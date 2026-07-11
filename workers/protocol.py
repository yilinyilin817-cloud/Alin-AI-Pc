"""Worker IPC 协议
MessagePack 帧 over stdin/stdout
帧格式: 4 字节大端长度前缀 + MessagePack 负载
"""
import sys
import struct
import msgpack


def read_message():
    """从 stdin 读取一条 MessagePack 消息"""
    len_bytes = sys.stdin.buffer.read(4)
    if not len_bytes or len(len_bytes) < 4:
        return None
    length = struct.unpack('>I', len_bytes)[0]
    data = sys.stdin.buffer.read(length)
    return msgpack.unpackb(data)


def send_message(msg: dict):
    """发送一条 MessagePack 消息到 stdout"""
    data = msgpack.packb(msg)
    sys.stdout.buffer.write(struct.pack('>I', len(data)))
    sys.stdout.buffer.write(data)
    sys.stdout.buffer.flush()


def send_chunk(chunk_id: int, text: str):
    """发送流式块"""
    send_message({"id": chunk_id, "chunk": text, "done": False})


def send_done(chunk_id: int):
    """发送流式结束"""
    send_message({"id": chunk_id, "chunk": "", "done": True})


def send_result(req_id: int, result):
    """发送结果（非流式）"""
    send_message({"id": req_id, "result": result, "error": None, "done": True})


def send_error(req_id: int, error: str):
    """发送错误"""
    send_message({"id": req_id, "result": None, "error": error, "done": True})


def handle_requests(handler):
    """主循环：读取请求并分发"""
    while True:
        req = read_message()
        if req is None:
            break
        req_id = req["id"]
        method = req["method"]
        params = req.get("params", {})
        try:
            handler(req_id, method, params)
        except Exception as e:
            send_error(req_id, str(e))
