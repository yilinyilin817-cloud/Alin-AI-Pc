"""翻译技能 - 使用 MyMemory 免费翻译 API"""
import sys
import json
import urllib.request
import urllib.parse


def run(args: dict) -> str:
    text = args.get("text", "")
    from_lang = args.get("from_lang", "auto")
    to_lang = args.get("to_lang", "en")

    if not text:
        return json.dumps({"error": "No text to translate"})

    # 语言代码标准化：中文 => zh-CN, 英文 => en-GB, etc.
    lang_map = {
        "zh": "zh-CN", "cn": "zh-CN", "chinese": "zh-CN",
        "en": "en-GB", "english": "en-GB",
        "ja": "ja", "jp": "ja", "japanese": "ja",
        "ko": "ko", "kr": "ko", "korean": "ko",
        "fr": "fr", "french": "fr",
        "de": "de", "german": "de",
        "es": "es", "spanish": "es",
        "pt": "pt", "portuguese": "pt",
        "ru": "ru", "russian": "ru",
        "ar": "ar", "arabic": "ar",
        "it": "it", "italian": "it",
        "th": "th", "thai": "th",
        "vi": "vi", "vietnamese": "vi",
    }

    from_lang = lang_map.get(from_lang.lower(), from_lang)
    to_lang = lang_map.get(to_lang.lower(), to_lang)

    # 处理 auto 检测
    if from_lang == "auto":
        lang_pair = f"|{to_lang}"
    else:
        lang_pair = f"{from_lang}|{to_lang}"

    try:
        url = "https://api.mymemory.translated.net/get"
        params = urllib.parse.urlencode({
            "q": text,
            "langpair": lang_pair,
        })
        full_url = f"{url}?{params}"

        req = urllib.request.Request(full_url, headers={
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"
        })

        with urllib.request.urlopen(req, timeout=15) as resp:
            data = json.loads(resp.read().decode("utf-8"))

        translated = data.get("responseData", {}).get("translatedText", "")
        detected_lang = data.get("responseData", {}).get("detectedLanguage", "")
        match_score = data.get("responseData", {}).get("match", 0)

        if not translated:
            return json.dumps({
                "status": "error",
                "message": "Translation returned empty result",
            })

        return json.dumps({
            "original": text,
            "translated": translated,
            "from_lang": from_lang,
            "to_lang": to_lang,
            "detected_lang": detected_lang if from_lang == "auto" else None,
            "confidence": round(match_score * 100, 1),
            "source": "MyMemory",
        })

    except Exception as e:
        return json.dumps({
            "status": "error",
            "message": f"Translation failed: {str(e)}",
            "original": text,
        })


if __name__ == "__main__":
    args = json.loads(sys.stdin.read())
    print(run(args))
