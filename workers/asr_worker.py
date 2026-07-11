"""ASR Worker - faster-whisper 语音识别
支持本地模型路径解析、自动设备检测、降级方案。
模型目录通过 AI_MODELS_DIR 环境变量传入。
"""
import sys
import os
import tempfile
from pathlib import Path
from protocol import handle_requests, send_result, send_error, send_message

_model = None
_model_path = None

MODEL_SIZE_MAP = {
    "whisper": "large-v3",
    "whisper-medium": "medium",
    "funasr": None,
}


def _get_models_dir() -> Path:
    env_dir = os.environ.get("AI_MODELS_DIR", "")
    if env_dir and Path(env_dir).exists():
        return Path(env_dir)
    cwd_models = Path.cwd() / ".." / "data" / "models"
    if cwd_models.exists():
        return cwd_models
    return Path(env_dir or "data/models")


def _resolve_model_path(model_id: str) -> str:
    """将 model_id 解析为 faster-whisper 可加载的路径或 size 名称。
    优先级：本地下载目录 -> HuggingFace 缓存（size 名称）
    """
    models_dir = _get_models_dir()

    local_dir = models_dir / model_id
    if local_dir.exists() and (local_dir / "model.bin").exists():
        return str(local_dir)

    if model_id in MODEL_SIZE_MAP:
        mapped = MODEL_SIZE_MAP[model_id]
        if mapped is None:
            return model_id
        local_mapped = models_dir / model_id
        if local_mapped.exists() and (local_mapped / "model.bin").exists():
            return str(local_mapped)
        return mapped

    hf_sizes = {"tiny", "base", "small", "medium", "large-v1", "large-v2", "large-v3", "large"}
    if model_id in hf_sizes:
        local_named = models_dir / model_id
        if local_named.exists() and (local_named / "model.bin").exists():
            return str(local_named)
        return model_id

    if Path(model_id).exists():
        return model_id

    return model_id


def _get_device():
    try:
        import torch
        if torch.cuda.is_available():
            return "cuda"
    except ImportError:
        pass
    return "cpu"


def _get_compute_type(device: str) -> str:
    if device == "cuda":
        return "float16"
    return "int8"


def _load_model(model_id: str = "whisper"):
    global _model, _model_path
    resolved = _resolve_model_path(model_id)
    if _model is not None and _model_path == resolved:
        return _model

    try:
        from faster_whisper import WhisperModel
    except ImportError:
        raise RuntimeError(
            "faster-whisper 未安装。请运行: pip install faster-whisper\n"
            "模型下载: python workers/download_models.py whisper"
        )

    device = _get_device()
    compute_type = _get_compute_type(device)
    print(f"[asr_worker] 加载模型 {model_id} -> {resolved} device={device} compute={compute_type}",
          file=sys.stderr, flush=True)

    try:
        _model = WhisperModel(resolved, device=device, compute_type=compute_type)
    except ValueError as e:
        if "float16" in str(e) or "CPU" in str(e):
            print(f"[asr_worker] float16 在 CPU 上不可用，降级 int8: {e}", file=sys.stderr, flush=True)
            _model = WhisperModel(resolved, device="cpu", compute_type="int8")
        else:
            raise
    _model_path = resolved
    return _model


def handle(req_id, method, params):
    if method == "shutdown":
        sys.exit(0)
    elif method == "ping":
        send_result(req_id, "pong")
    elif method == "transcribe":
        transcribe(req_id, params)
    elif method == "health":
        send_result(req_id, _get_health())
    else:
        send_message({"id": req_id, "result": None, "error": f"Unknown: {method}", "done": True})


def _get_health():
    try:
        _load_model()
        return {
            "status": "ready",
            "model": _model_path,
            "device": _get_device(),
            "models_dir": str(_get_models_dir()),
        }
    except Exception as e:
        return {"status": "error", "error": str(e)}


def transcribe(req_id, params):
    audio_data = params.get("audio_bytes")
    audio_path = params.get("audio_path")
    model_id = params.get("model_size", "whisper")
    language = params.get("language")

    if not audio_data and not audio_path:
        send_error(req_id, "No audio data provided (need audio_bytes or audio_path)")
        return

    tmp_path = None
    if audio_data:
        with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as f:
            if isinstance(audio_data, list):
                audio_data = bytes(audio_data)
            f.write(audio_data)
            tmp_path = f.name
        audio_path = tmp_path

    try:
        model = _load_model(model_id)

        transcribe_opts = {"vad_filter": True}
        if language:
            transcribe_opts["language"] = language

        segments, info = model.transcribe(audio_path, **transcribe_opts)

        text_parts = []
        seg_list = []
        for seg in segments:
            text_parts.append(seg.text)
            seg_list.append({
                "start": round(seg.start, 3),
                "end": round(seg.end, 3),
                "text": seg.text,
            })

        full_text = " ".join(text_parts).strip()
        send_result(req_id, {
            "text": full_text,
            "segments": seg_list,
            "language": info.language,
            "language_probability": info.language_probability,
        })

    except RuntimeError as e:
        send_error(req_id, str(e))
    except Exception as e:
        send_error(req_id, f"ASR 转录失败: {e}")
    finally:
        if tmp_path and os.path.exists(tmp_path):
            try:
                os.unlink(tmp_path)
            except OSError:
                pass


if __name__ == "__main__":
    handle_requests(handle)
