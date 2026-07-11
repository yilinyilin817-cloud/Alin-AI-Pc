"""Embedding Worker - bge-m3 / sentence-transformers"""
import sys
from protocol import handle_requests, send_result, send_message


_cache = {}


def get_model(model_name: str):
    if model_name not in _cache:
        from sentence_transformers import SentenceTransformer
        _cache[model_name] = SentenceTransformer(model_name, device="cpu")
    return _cache[model_name]


def handle(req_id, method, params):
    if method == "shutdown":
        sys.exit(0)
    elif method == "ping":
        send_result(req_id, {"status": "ok"})
    elif method == "embed":
        texts = params.get("texts", [])
        model_name = params.get("model_name", "BAAI/bge-m3")
        model_path = params.get("model_path")  # 支持本地路径
        if not texts:
            send_message({"id": req_id, "result": [], "error": None, "done": True})
            return
        try:
            search_name = model_path if model_path else model_name
            model = get_model(search_name)
            vecs = model.encode(texts, normalize_embeddings=True).tolist()
            send_result(req_id, vecs)
        except ImportError:
            import random
            dim = 1024
            vecs = [[random.gauss(0, 0.1) for _ in range(dim)] for _ in texts]
            send_message({
                "id": req_id,
                "result": vecs,
                "error": "sentence-transformers not installed; using random placeholder",
                "done": True,
            })
    else:
        send_message({"id": req_id, "result": None, "error": f"Unknown: {method}", "done": True})


if __name__ == "__main__":
    handle_requests(handle)
