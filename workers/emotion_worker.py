"""Emotion Worker - Wav2Vec2 语音情绪识别"""
import sys
import json
from protocol import handle_requests, send_result, send_error, send_message


def handle(req_id, method, params):
    if method == "shutdown":
        sys.exit(0)
    elif method == "recognize_emotion":
        recognize(req_id, params)
    else:
        send_message({"id": req_id, "result": None, "error": f"Unknown: {method}"})


def recognize(req_id, params):
    """识别音频中的情绪"""
    audio_data = params.get("audio_bytes", [])
    text = params.get("text", "")

    if isinstance(audio_data, list):
        audio_data = bytes(audio_data)

    result = {"emotion": "neutral", "valence": 0.0, "arousal": 0.3}

    # 语音情绪识别（若可用）
    if audio_data:
        try:
            import torch
            import torchaudio
            from transformers import Wav2Vec2ForSequenceClassification, Wav2Vec2Processor

            model = Wav2Vec2ForSequenceClassification.from_pretrained(
                "facebook/wav2vec2-base"
            )
            processor = Wav2Vec2Processor.from_pretrained(
                "facebook/wav2vec2-base"
            )

            import tempfile
            with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as f:
                f.write(audio_data)
                tmp_path = f.name

            try:
                speech, sr = torchaudio.load(tmp_path)
                if sr != 16000:
                    resampler = torchaudio.transforms.Resample(sr, 16000)
                    speech = resampler(speech)
                inputs = processor(speech, sampling_rate=16000, return_tensors="pt")
                with torch.no_grad():
                    outputs = model(**inputs).logits
                pred = torch.argmax(outputs, dim=1).item()
                emotions = ["neutral", "happy", "sad", "angry", "fearful"]
                result["emotion"] = emotions[pred] if pred < len(emotions) else "neutral"
                result["valence"] = 0.3 if result["emotion"] == "happy" else -0.2
                result["arousal"] = 0.7 if result["emotion"] in ("angry", "happy") else 0.3
            finally:
                import os
                os.unlink(tmp_path)
        except ImportError:
            pass  # 静默降级

    # 文本情绪分析（简易字典法）
    if text:
        positive_words = ["开心", "高兴", "快乐", "喜欢", "棒", "好", "爱", "幸福", "哈哈", "赞"]
        negative_words = ["难过", "伤心", "生气", "讨厌", "烦", "累", "痛苦", "焦虑", "怕"]

        pos_count = sum(1 for w in positive_words if w in text)
        neg_count = sum(1 for w in negative_words if w in text)

        if pos_count > neg_count:
            result["emotion"] = "happy"
            result["valence"] = 0.6
            result["arousal"] = 0.5
        elif neg_count > pos_count:
            result["emotion"] = "sad"
            result["valence"] = -0.5
            result["arousal"] = 0.4

    send_result(req_id, result)


if __name__ == "__main__":
    handle_requests(handle)
