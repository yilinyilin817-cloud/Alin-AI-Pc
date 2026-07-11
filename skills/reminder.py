"""提醒设置技能"""
import sys
import json
import datetime


def run(args: dict) -> str:
    content = args.get("content", "")
    fire_at = args.get("fire_at", "")
    repeat = args.get("repeat", "none")

    # 提醒实际由 Rust 后端持久化到 SQLite。
    # 此 Python 脚本仅做格式校验和确认。
    try:
        if fire_at:
            datetime.datetime.fromisoformat(fire_at)
    except ValueError:
        return json.dumps({"status": "error", "message": "Invalid time format. Use ISO 8601."})

    return json.dumps({
        "status": "created",
        "content": content,
        "fire_at": fire_at,
        "repeat": repeat,
        "message": f"提醒已设置：{content}（{fire_at}）",
    })


if __name__ == "__main__":
    args = json.loads(sys.stdin.read())
    print(run(args))
