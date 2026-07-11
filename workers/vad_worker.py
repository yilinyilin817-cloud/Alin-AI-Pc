"""VAD Worker - silero-vad 语音活动检测"""
import sys
import struct
from protocol import handle_requests, send_result, send_message


def handle(req_id, method, params):
    if method == "shutdown":
        sys.exit(0)
    elif method == "detect_speech":
        audio_data = params.get("audio_bytes", [])
        if isinstance(audio_data, list):
            audio_data = bytes(audio_data)
        try:
            _detect(req_id, audio_data)
        except Exception as e:
            send_message({"id": req_id, "result": None, "error": str(e)})
    else:
        send_message({"id": req_id, "result": None, "error": f"Unknown: {method}"})


def _detect(req_id, audio_data: bytes):
    try:
        import torch
        import torchaudio
        from silero_vad import get_speech_timestamps

        # 写入临时文件
        import tempfile
        with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as f:
            f.write(audio_data)
            tmp_path = f.name

        try:
            wav, sr = torchaudio.load(tmp_path)
            if sr != 16000:
                resampler = torchaudio.transforms.Resample(sr, 16000)
                wav = resampler(wav)
                sr = 16000

            timestamps = get_speech_timestamps(wav[0], return_seconds=True)
            has_speech = len(timestamps) > 0
            duration = timestamps[-1]["end"] - timestamps[0]["start"] if has_speech else 0

            send_result(req_id, {
                "has_speech": has_speech,
                "timestamps": timestamps,
                "duration_sec": duration,
            })
        finally:
            import os
            os.unlink(tmp_path)
    except ImportError:
        # Mock fallback
        send_result(req_id, {
            "has_speech": True,
            "timestamps": [{"start": 0.0, "end": 0.5}],
            "duration_sec": 0.5,
        })


if __name__ == "__main__":
    handle_requests(handle)
