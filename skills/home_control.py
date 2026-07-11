"""智能家居控制技能 - MQTT"""
import sys
import json


def run(args: dict) -> str:
    device = args.get("device", "")
    action = args.get("action", "toggle")
    value = args.get("value")

    try:
        import paho.mqtt.client as mqtt
        client = mqtt.Client()
        client.connect("127.0.0.1", 1883, 60)
        topic = f"home/{device}/command"
        payload = json.dumps({"action": action, "value": value})
        client.publish(topic, payload)
        client.disconnect()
        return json.dumps({"status": "sent", "device": device, "action": action})
    except ImportError:
        return json.dumps({
            "status": "unavailable",
            "message": "MQTT not configured (pip install paho-mqtt)",
        })
    except Exception as e:
        return json.dumps({"status": "error", "message": str(e)})


if __name__ == "__main__":
    args = json.loads(sys.stdin.read())
    print(run(args))
