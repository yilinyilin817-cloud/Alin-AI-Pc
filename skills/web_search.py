"""网络搜索技能 - DuckDuckGo HTML（免 API Key）"""
import sys
import json
import urllib.parse
import urllib.request
import re


def run(args: dict) -> str:
    query = args.get("query", "")
    count = args.get("count", 5)

    if not query:
        return json.dumps({"error": "No query"})

    try:
        encoded = urllib.parse.quote(query)
        url = f"https://html.duckduckgo.com/html/?q={encoded}"
        req = urllib.request.Request(url, headers={
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
        })
        with urllib.request.urlopen(req, timeout=15) as resp:
            html = resp.read().decode("utf-8")

        # 简易结果提取（非完美但够用）
        results = []
        for match in re.finditer(
            r'<a rel="nofollow" href="(.*?)".*?class="result__a">(.*?)</a>',
            html, re.DOTALL
        ):
            url = match.group(1)
            title = re.sub(r"<[^>]+>", "", match.group(2)).strip()
            results.append({"title": title, "url": url})
            if len(results) >= count:
                break

        return json.dumps({"query": query, "results": results, "count": len(results)})
    except Exception as e:
        return json.dumps({"query": query, "error": str(e), "fallback": "Search unavailable"})


if __name__ == "__main__":
    args = json.loads(sys.stdin.read())
    print(run(args))
