"""天气查询技能 - 使用 wttr.in（免 API Key）"""
import sys
import json
import urllib.request


def run(args: dict) -> str:
    city = args.get("city", "北京")
    try:
        url = f"https://wttr.in/{city}?format=%C+%t+%h+%w"
        with urllib.request.urlopen(url, timeout=10) as resp:
            data = resp.read().decode("utf-8").strip()
        return json.dumps({"city": city, "weather": data, "source": "wttr.in"})
    except Exception as e:
        return json.dumps({"city": city, "error": str(e), "fallback": "晴 28°C"})


if __name__ == "__main__":
    args = json.loads(sys.stdin.read())
    print(run(args))
