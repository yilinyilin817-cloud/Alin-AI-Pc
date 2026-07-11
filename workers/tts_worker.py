"""TTS Worker - pyttsx3 离线 TTS (主引擎) + CosyVoice/ChatTTS 高质量可选
自动降级：CosyVoice → ChatTTS → pyttsx3 → 错误提示
模型目录通过 AI_MODELS_DIR 环境变量传入
"""
import sys
import os
import base64
import tempfile
from pathlib import Path
from protocol import handle_requests, send_result, send_error, send_message

_engine = None
_cosyvoice_cls = None
_cosyvoice_instance = None
_cosyvoice_model_dir = None
_piper_model = None
_available = None


def _get_models_dir() -> Path:
    env_dir = os.environ.get("AI_MODELS_DIR", "")
    if env_dir and Path(env_dir).exists():
        return Path(env_dir)
    cwd_models = Path.cwd() / ".." / "data" / "models"
    if cwd_models.exists():
        return cwd_models
    return Path(env_dir or "data/models")


def _find_cosyvoice_model() -> str | None:
    models_dir = _get_models_dir()
    candidates = [
        models_dir / "cosyvoice",
        models_dir / "CosyVoice2-0.5B",
        models_dir / "cosyvoice-sft",
        models_dir / "CosyVoice-300M-SFT",
        Path("pretrained_models/CosyVoice2-0.5B"),
        Path("pretrained_models/CosyVoice-300M-SFT"),
    ]
    for d in candidates:
        if d.exists():
            if (d / "cosyvoice.yaml").exists() or (d / "llm.pt").exists() or d.name.startswith("CosyVoice"):
                return str(d)
    return None


def _get_engine():
    global _engine
    if _engine is not None:
        return _engine
    try:
        import pyttsx3
        _engine = pyttsx3.init()
        rate = _engine.getProperty("rate")
        _engine.setProperty("rate", rate - 10)
        try:
            voices = _engine.getProperty("voices")
            for v in voices:
                langs = getattr(v, "languages", [])
                vid = getattr(v, "id", "")
                if any("zh" in str(l).lower() or "chinese" in str(v.name).lower() for l in langs) or \
                   "chinese" in str(getattr(v, "name", "")).lower() or "zh" in vid.lower():
                    _engine.setProperty("voice", vid)
                    break
        except Exception:
            pass
        return _engine
    except ImportError:
        raise RuntimeError("pyttsx3 未安装。请运行: pip install pyttsx3")
    except Exception as e:
        raise RuntimeError(f"pyttsx3 初始化失败: {e}")


def _load_cosyvoice():
    global _cosyvoice_instance, _cosyvoice_cls, _cosyvoice_model_dir
    if _cosyvoice_instance is not None:
        return _cosyvoice_instance

    model_dir = _find_cosyvoice_model()
    if not model_dir:
        return None

    try:
        try:
            from cosyvoice.cli.cosyvoice import CosyVoice2
            _cosyvoice_cls = "v2"
            _cosyvoice_instance = CosyVoice2(model_dir, load_jit=False, load_trt=False, fp16=False)
        except (ImportError, Exception):
            try:
                from cosyvoice.cli.cosyvoice import CosyVoice
                _cosyvoice_cls = "v1"
                _cosyvoice_instance = CosyVoice(model_dir)
            except Exception:
                return None
        _cosyvoice_model_dir = model_dir
        return _cosyvoice_instance
    except ImportError:
        return None
    except Exception as e:
        print(f"[tts_worker] CosyVoice 加载失败 ({model_dir}): {e}", file=sys.stderr, flush=True)
        return None


def _load_piper():
    global _piper_model
    if _piper_model is not None:
        return _piper_model
    try:
        from piper import PiperVoice
        models_dir = _get_models_dir()
        model_path = None
        for candidate in [
            models_dir / "piper" / "zh" / "zh_CN" / "medium" / "zh_CN-medium.onnx",
            models_dir / "piper" / "zh_CN-medium.onnx",
        ]:
            if candidate.exists():
                model_path = candidate
                break
        if model_path:
            config_path = model_path.with_suffix(".onnx.json")
            if not config_path.exists():
                config_path = None
            _piper_model = PiperVoice.load(str(model_path), config_path=str(config_path) if config_path else None)
            return _piper_model
    except ImportError:
        pass
    except Exception as e:
        print(f"[tts_worker] Piper 加载失败: {e}", file=sys.stderr, flush=True)
    return None


def _detect_backends():
    global _available
    if _available is not None:
        return _available
    backends = []
    try:
        _get_engine()
        backends.append("pyttsx3")
    except Exception:
        pass

    if _load_cosyvoice() is not None:
        backends.append("cosyvoice")

    if _load_piper() is not None:
        backends.append("piper")

    _available = backends
    return backends


def handle(req_id, method, params):
    if method == "shutdown":
        sys.exit(0)
    elif method == "ping":
        send_result(req_id, "pong")
    elif method == "synthesize":
        synthesize(req_id, params)
    elif method == "health":
        send_result(req_id, {
            "backends": _detect_backends(),
            "status": "ready" if _detect_backends() else "no_backend",
            "models_dir": str(_get_models_dir()),
            "cosyvoice_model": _find_cosyvoice_model(),
        })
    else:
        send_message({"id": req_id, "result": None, "error": f"Unknown: {method}", "done": True})


def _synthesize_with_cosyvoice(text: str) -> bytes | None:
    cosy = _load_cosyvoice()
    if cosy is None:
        return None
    try:
        import torch
        with torch.no_grad():
            if _cosyvoice_cls == "v2":
                outputs = cosy.inference_sft(text, stream=False, speed=1.0)
            else:
                outputs = cosy.inference_sft(tts_text=text, stream=False)
        audio = None
        if isinstance(outputs, list) and len(outputs) > 0:
            outputs = outputs[0]
        if hasattr(outputs, "get"):
            audio = outputs.get("tts_speech")
        elif isinstance(outputs, dict):
            audio = outputs.get("tts_speech")
        if audio is None:
            return None
        if isinstance(audio, torch.Tensor):
            audio = audio.cpu().numpy()
        import numpy as np
        if audio.ndim > 1:
            audio = audio.squeeze()
        audio = (audio * 32767).clip(-32768, 32767).astype("int16")
        sample_rate = getattr(cosy, "sample_rate", 22050) or 22050
        return _make_wav(audio.tobytes(), sample_rate=sample_rate)
    except Exception as e:
        print(f"[tts_worker] CosyVoice 推理失败: {e}", file=sys.stderr, flush=True)
        return None


def _synthesize_with_piper(text: str) -> bytes | None:
    voice = _load_piper()
    if voice is None:
        return None
    try:
        import wave
        import io
        buf = io.BytesIO()
        with wave.open(buf, "wb") as wav_file:
            wav_file.setnchannels(1)
            wav_file.setsampwidth(2)
            wav_file.setframerate(voice.config.sample_rate)
            voice.synthesize(text, wav_file)
        return buf.getvalue()
    except Exception as e:
        print(f"[tts_worker] Piper 推理失败: {e}", file=sys.stderr, flush=True)
        return None


def _synthesize_with_pyttsx3(text: str) -> bytes | None:
    try:
        engine = _get_engine()
        with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as f:
            tmp_path = f.name
        engine.save_to_file(text, tmp_path)
        engine.runAndWait()
        with open(tmp_path, "rb") as f:
            wav_data = f.read()
        try:
            os.unlink(tmp_path)
        except OSError:
            pass
        return wav_data
    except Exception as e:
        print(f"[tts_worker] pyttsx3 合成失败: {e}", file=sys.stderr, flush=True)
        return None


def synthesize(req_id, params):
    text = params.get("text", "")
    backend = params.get("backend", "auto")
    voice_id = params.get("voice_id", "")

    if not text:
        send_error(req_id, "No text to synthesize")
        return

    backends = _detect_backends()
    if not backends:
        send_error(req_id, "没有可用的 TTS 后端。请安装 pyttsx3: pip install pyttsx3")
        return

    if backend != "auto":
        order = [backend]
    else:
        order = []
        if "cosyvoice" in backends:
            order.append("cosyvoice")
        if "piper" in backends:
            order.append("piper")
        if "pyttsx3" in backends:
            order.append("pyttsx3")

    wav_bytes = None
    used_backend = None
    for b in order:
        if b == "cosyvoice" and "cosyvoice" in backends:
            wav_bytes = _synthesize_with_cosyvoice(text)
        elif b == "piper" and "piper" in backends:
            wav_bytes = _synthesize_with_piper(text)
        elif b == "pyttsx3" and "pyttsx3" in backends:
            wav_bytes = _synthesize_with_pyttsx3(text)
        if wav_bytes is not None:
            used_backend = b
            break

    if wav_bytes is None:
        send_error(req_id, "所有 TTS 后端均合成失败，请检查依赖安装")
        return

    audio_b64 = base64.b64encode(wav_bytes).decode()
    send_result(req_id, {"audio_data": audio_b64, "format": "wav", "backend": used_backend})


def _make_wav(raw_samples: bytes, sample_rate: int = 22050, channels: int = 1, bits: int = 16) -> bytes:
    import io
    import struct

    buf = io.BytesIO()
    byte_rate = sample_rate * channels * (bits // 8)
    block_align = channels * (bits // 8)

    buf.write(b"RIFF")
    buf.write(struct.pack("<I", 36 + len(raw_samples)))
    buf.write(b"WAVE")
    buf.write(b"fmt ")
    buf.write(struct.pack("<I", 16))
    buf.write(struct.pack("<H", 1))
    buf.write(struct.pack("<H", channels))
    buf.write(struct.pack("<I", sample_rate))
    buf.write(struct.pack("<I", byte_rate))
    buf.write(struct.pack("<H", block_align))
    buf.write(struct.pack("<H", bits))
    buf.write(b"data")
    buf.write(struct.pack("<I", len(raw_samples)))
    buf.write(raw_samples)

    return buf.getvalue()


if __name__ == "__main__":
    handle_requests(handle)
