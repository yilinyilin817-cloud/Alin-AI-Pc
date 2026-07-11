#!/usr/bin/env python3
"""
模型下载脚本：从 HuggingFace 或 ModelScope 下载模型到 data/models/
用法：
  python download_models.py <model_id> [--output <dir>] [--backend auto|huggingface|modelscope]
  python download_models.py --list
  python download_models.py --all

进度通过 stdout 行输出，格式为: PROGRESS:<pct>
Rust 后端会解析此格式来更新下载进度。
"""
import os
import sys
import json
import argparse
from pathlib import Path

# ─── 模型注册表 ─────────────────────────────────────────

MODELS = {
    # ── LLM（多模态/通用） ──
    "gemma4-12b": {
        "name": "Gemma 2 9B Unified",
        "type": "llm",
        "source_hf": "unsloth/gemma-2-9b-it-GGUF",
        "files": ["gemma-2-9b-it-Q4_K_M.gguf"],
        "size_gb": 5.4,
        "vram_gb": "8-10",
        "description": "Google 最新 Gemma 2 架构，极高性能（GGUF 4-bit）",
        "ollama_tag": "gemma2:9b",
    },
    "gemma4-e4b": {
        "name": "Gemma 2 2B Unified",
        "type": "llm",
        "source_hf": "unsloth/gemma-2-2b-it-GGUF",
        "files": ["gemma-2-2b-it-Q4_K_M.gguf"],
        "size_gb": 1.6,
        "vram_gb": "4",
        "description": "极致轻量 Gemma 2，适合低显存",
        "ollama_tag": "gemma2:2b",
    },
    "llama4-scout": {
        "name": "Llama 3.1 8B Instruct",
        "type": "llm",
        "source_hf": "unsloth/Meta-Llama-3.1-8B-Instruct-GGUF",
        "files": ["Meta-Llama-3.1-8B-Instruct-Q4_K_M.gguf"],
        "size_gb": 4.9,
        "vram_gb": "6-8",
        "description": "Meta 官方最新 Llama 3.1（GGUF 4-bit）",
        "ollama_tag": "llama3.1:8b",
    },
    "qwen3-vl-8b": {
        "name": "Qwen 2-VL 7B Instruct",
        "type": "llm",
        "source_hf": "Qwen/Qwen2-VL-7B-Instruct-GGUF",
        "files": ["qwen2-vl-7b-instruct-q4_k_m.gguf"],
        "size_gb": 4.5,
        "vram_gb": "6-8",
        "description": "阿里最新多模态模型，支持视觉理解",
        "ollama_tag": "qwen2-vl:7b",
    },
    "phi4-multimodal": {
        "name": "Phi-3.5 Vision Instruct",
        "type": "llm",
        "source_hf": "microsoft/Phi-3.5-vision-instruct",
        "snapshot": True,
        "size_gb": 8.0,
        "vram_gb": "8",
        "description": "微软最新多模态，支持视觉理解",
        "ollama_tag": "phi3.5:vision",
    },
    # ── ASR ──
    "whisper": {
        "name": "faster-whisper-large-v3",
        "type": "asr",
        "source_hf": "guillaumekwn/faster-whisper-large-v3",
        "source_ms": "guillaumekwn/faster-whisper-large-v3",
        "files": ["model.bin", "config.json", "tokenizer.json"],
        "size_gb": 1.5,
        "vram_gb": "2-3",
        "description": "高精度语音识别（最佳质量）",
    },
    "whisper-medium": {
        "name": "faster-whisper-medium",
        "type": "asr",
        "source_hf": "guillaumekwn/faster-whisper-medium",
        "source_ms": "guillaumekwn/faster-whisper-medium",
        "files": ["model.bin", "config.json", "tokenizer.json"],
        "size_gb": 0.8,
        "vram_gb": "1-2",
        "description": "中等精度语音识别（速度优先）",
    },
    "funasr": {
        "name": "FunASR Paraformer-zh",
        "type": "asr",
        "source_ms": "iic/speech_paraformer-large_asr_nat-zh-cn-16k-common-vocab8404-pytorch",
        "files": ["model.pt", "config.yaml", "am.mvn"],
        "size_gb": 1.0,
        "vram_gb": "1-2",
        "description": "阿里达摩院中文语音识别，中文准确率最优",
    },
    # ── TTS ──
    "cosyvoice": {
        "name": "CosyVoice2-0.5B",
        "type": "tts",
        "source_hf": "FunAudioLLM/CosyVoice2-0.5B",
        "source_ms": "iic/CosyVoice2-0.5B",
        "snapshot": True,
        "size_gb": 1.5,
        "vram_gb": "2-4",
        "description": "阿里通义 CosyVoice2，零样本语音克隆 + 情感控制（高质量）",
        "model_dir": "CosyVoice2-0.5B",
    },
    "cosyvoice-sft": {
        "name": "CosyVoice-300M-SFT",
        "type": "tts",
        "source_hf": "FunAudioLLM/CosyVoice-300M-SFT",
        "source_ms": "iic/CosyVoice-300M-SFT",
        "snapshot": True,
        "size_gb": 1.2,
        "vram_gb": "2-3",
        "description": "CosyVoice SFT 版本，预置音色，适合快速推理",
        "model_dir": "CosyVoice-300M-SFT",
    },
    "pyttsx3": {
        "name": "pyttsx3 本地 TTS（免下载）",
        "type": "tts",
        "size_gb": 0,
        "vram_gb": "0",
        "description": "使用操作系统内置语音引擎（Windows SAPI5 / macOS NSSpeech / Linux espeak），无需下载模型",
        "builtin": True,
    },
    # ── Embedding ──
    "bge-m3": {
        "name": "bge-m3",
        "type": "embedding",
        "source_hf": "BAAI/bge-m3",
        "source_ms": "Xorbits/bge-m3",
        "files": ["model.safetensors", "tokenizer.json", "config.json"],
        "size_gb": 1.2,
        "vram_gb": "2",
        "description": "中英多语言 Embedding，1024 维",
    },
    "bge-small": {
        "name": "bge-small-zh-v1.5",
        "type": "embedding",
        "source_hf": "BAAI/bge-small-zh-v1.5",
        "source_ms": "iic/nlp_corom_sentence-embedding_chinese-small",
        "files": ["model.safetensors", "tokenizer.json", "config.json"],
        "size_gb": 0.1,
        "vram_gb": "0.5",
        "description": "轻量中文 Embedding，512 维（低资源）",
    },
    "jina-v3": {
        "name": "jina-embeddings-v3",
        "type": "embedding",
        "source_hf": "jinaai/jina-embeddings-v3",
        "files": ["model.safetensors", "tokenizer.json", "config.json"],
        "size_gb": 1.0,
        "vram_gb": "2",
        "description": "多语言高性能 Embedding，1024 维",
    },
    # ── TTS 补充（轻量/对话场景） ──
    "chattts": {
        "name": "ChatTTS",
        "type": "tts",
        "source_hf": "2noise/ChatTTS",
        "source_ms": "AI-ModelScope/ChatTTS",
        "snapshot": True,
        "size_gb": 1.2,
        "vram_gb": "2-3",
        "description": "对话场景优化 TTS，自然韵律，支持笑声/停顿",
    },
    "piper": {
        "name": "Piper TTS (onnx)",
        "type": "tts",
        "source_hf": "rhasspy/piper-voices",
        "source_ms": None,
        "files": ["zh/zh_CN/medium/zh_CN-medium.onnx", "zh/zh_CN/medium/zh_CN-medium.onnx.json"],
        "size_gb": 0.08,
        "vram_gb": "0.2",
        "description": "超轻量 ONNX TTS，免 GPU，CPU 实时合成",
    },
}

def check_deps(backend="huggingface"):
    if backend == "modelscope":
        try:
            import modelscope
            return True
        except ImportError:
            return False
    try:
        import huggingface_hub
        import tqdm
        return True
    except ImportError:
        return False

def report_progress(pct: int):
    print(f"PROGRESS:{pct}")
    sys.stdout.flush()

def report_error(msg: str):
    print(f"ERROR:{msg}")
    sys.stdout.flush()

def report_done():
    print("DONE")
    sys.stdout.flush()

def report_loadcheck(status: str, detail: str = ""):
    """模型加载校验结果"""
    msg = f"LOADCHECK:{status}"
    if detail:
        msg += f":{detail}"
    print(msg)
    sys.stdout.flush()

def _is_complete_snapshot(d: Path) -> bool:
    """检查 snapshot 下载的模型目录是否完整（不仅仅是 .cache 或空目录）。
    规则：至少有一个非隐藏文件/目录（不以 . 开头），且不处于下载中状态。
    """
    if not d.exists() or not d.is_dir():
        return False
    entries = list(d.iterdir())
    if not entries:
        return False
    # 忽略隐藏文件/目录（如 .cache, .huggingface 等）
    real_entries = [e for e in entries if not e.name.startswith(".")]
    if not real_entries:
        return False
    # 检查是否还在下载中（incomplete marker）
    for e in entries:
        if e.name.endswith(".incomplete") or e.name.endswith(".lock"):
            return False
    return True


def download_model(model_id: str, output_dir: str, backend: str = "auto"):
    if model_id not in MODELS:
        report_error(f"未知模型: {model_id}")
        return False

    info = MODELS[model_id]

    # 内置模型，无需下载
    if info.get("builtin"):
        print(f"✓ {info['name']} 为内置引擎，无需下载")
        report_done()
        return True

    target_dir = Path(output_dir) / model_id
    target_dir.mkdir(parents=True, exist_ok=True)

    # 完整性检查
    already_complete = False
    if not info.get("snapshot"):
        required_files = info.get("files", [])
        if required_files:
            missing_files = [f for f in required_files if not (target_dir / f).exists()]
            if not missing_files:
                already_complete = True
        else:
            already_complete = _is_complete_snapshot(target_dir)
    else:
        already_complete = _is_complete_snapshot(target_dir)

    if already_complete:
        print(f"✓ {model_id} 已完整存在，跳过下载")
        # 对已存在模型也做加载校验
        _validate_download(model_id, info, target_dir)
        report_done()
        return True

    if backend == "auto":
        backend = "modelscope" if info.get("source_ms") else "huggingface"

    print(f"下载 {info['name']} via {backend}")

    # 实际下载逻辑
    success = _do_download(model_id, info, target_dir, backend)

    if success:
        _validate_download(model_id, info, target_dir)
        report_done()
        return True
    return False

def _do_download(model_id, info, target_dir, backend):
    if backend == "modelscope":
        ms_source = info.get("source_ms")
        if not ms_source:
            print("该模型无 ModelScope 源，切换到 HuggingFace...")
            return _do_download(model_id, info, target_dir, "huggingface")
        if not check_deps("modelscope"):
            report_error("需要 modelscope: pip install modelscope")
            return False
        try:
            from modelscope.hub.snapshot_download import snapshot_download
            report_progress(10)
            snapshot_download(ms_source, local_dir=str(target_dir))
            report_progress(100)
            return True
        except Exception as e:
            if info.get("source_hf"):
                print(f"ModelScope 失败，重试 HuggingFace...")
                return _do_download(model_id, info, target_dir, "huggingface")
            report_error(str(e))
            return False

    # HuggingFace Backend
    if not check_deps("huggingface"):
        report_error("需要 huggingface_hub 和 tqdm")
        return False

    try:
        from huggingface_hub import hf_hub_download, snapshot_download as hf_snapshot_download
        from tqdm import tqdm

        class ProgressLogger(tqdm):
            def __init__(self, *args, **kwargs):
                self.last_reported = -1
                super().__init__(*args, **kwargs)
            def update(self, n=1):
                super().update(n)
                if self.total:
                    pct = int((self.n / self.total) * 100)
                    if pct != self.last_reported:
                        report_progress(pct)
                        self.last_reported = pct

        if info.get("snapshot"):
            hf_snapshot_download(repo_id=info["source_hf"], local_dir=str(target_dir), tqdm_class=ProgressLogger)
        else:
            total = len(info["files"])
            for idx, f in enumerate(info["files"]):
                hf_hub_download(repo_id=info["source_hf"], filename=f, local_dir=str(target_dir), tqdm_class=ProgressLogger)
                report_progress(((idx + 1) * 100) // total)

        return True
    except Exception as e:
        ms_source = info.get("source_ms")
        if backend != "modelscope" and ms_source:
            print(f"HuggingFace 失败，重试 ModelScope...")
            return _do_download(model_id, info, target_dir, "modelscope")
        report_error(str(e))
        return False

def _validate_download(model_id, info, target_dir):
    """下载后尝试加载模型，验证可用性"""
    model_type = info.get("type", "")
    try:
        if model_type == "asr":
            report_progress(98)
            try:
                from faster_whisper import WhisperModel
                # 尝试以本地路径加载第一个文件所在目录
                WhisperModel(str(target_dir), device="cpu", compute_type="int8")
                report_loadcheck("OK", "faster-whisper 模型加载成功")
            except Exception as e:
                report_loadcheck("WARN", f"ASR 加载测试失败(可能需GPU): {str(e)[:80]}")
        elif model_type == "tts":
            report_progress(98)
            try:
                import pyttsx3
                engine = pyttsx3.init()
                engine.stop()
                report_loadcheck("OK", "pyttsx3 TTS 引擎就绪")
            except Exception as e:
                report_loadcheck("WARN", f"TTS 加载测试失败: {str(e)[:80]}")
        elif model_type == "embedding":
            report_progress(98)
            try:
                from sentence_transformers import SentenceTransformer
                SentenceTransformer(str(target_dir), device="cpu")
                report_loadcheck("OK", "Embedding 模型加载成功")
            except Exception as e:
                report_loadcheck("WARN", f"Embedding 加载测试失败: {str(e)[:80]}")
        else:
            report_loadcheck("SKIP", f"类型 {model_type} 无需加载校验")
    except ImportError as e:
        report_loadcheck("SKIP", f"依赖未安装: {str(e)[:60]}")
    except Exception as e:
        report_loadcheck("FAIL", str(e)[:120])

def list_models():
    print(f"{'ID':<22s} {'类型':<10s} {'大小':<8s} 名称")
    print("-" * 65)
    for mid, info in sorted(MODELS.items()):
        type_cn = {"llm": "LLM", "asr": "ASR", "tts": "TTS", "embedding": "Embedding"}.get(info.get("type", ""), info.get("type", ""))
        print(f"{mid:<22s} {type_cn:<10s} {info.get('size_gb', 0):>4.1f}GB  {info['name']}")

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("model", nargs="?")
    parser.add_argument("--list", action="store_true")
    parser.add_argument("--all", action="store_true")
    parser.add_argument("--output", "-o", default="data/models")
    parser.add_argument("--backend", default="auto", choices=["auto", "huggingface", "modelscope"])
    args = parser.parse_args()

    if args.list:
        list_models()
    elif args.all:
        failed = []
        for model_id, info in MODELS.items():
            if info.get("builtin"):
                continue
            print(f"\n=== 下载 {model_id} ({info['name']}) ===")
            if not download_model(model_id, args.output, args.backend):
                failed.append(model_id)
        if failed:
            print(f"\n失败模型: {', '.join(failed)}", file=sys.stderr)
            sys.exit(1)
    elif args.model:
        success = download_model(args.model, args.output, args.backend)
        sys.exit(0 if success else 1)
    else:
        parser.print_help()

if __name__ == "__main__":
    main()
