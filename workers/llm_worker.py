"""LLM Worker - llama.cpp + Ollama 适配器"""
import sys
import os
from protocol import read_message, send_chunk, send_done, send_message

BACKEND = os.environ.get("LLM_BACKEND", "llama_cpp")


def handle_llama_cpp(req_id, params):
    """使用 llama-cpp-python 推理"""
    from llama_cpp import Llama

    model_path = params.get("model_path", "")
    messages = params.get("messages", [])
    temperature = params.get("temperature", 0.7)
    max_tokens = params.get("max_tokens", 2048)

    llm = Llama(
        model_path=model_path,
        n_gpu_layers=-1,
        n_ctx=params.get("n_ctx", 16384),
        verbose=False,
    )

    response = llm.create_chat_completion(
        messages=messages,
        temperature=temperature,
        max_tokens=max_tokens,
        stream=True,
    )

    for chunk in response:
        delta = chunk["choices"][0]["delta"]
        if "content" in delta and delta["content"]:
            send_chunk(req_id, delta["content"])

        # 检测 tool_calls
        if "tool_calls" in delta and delta["tool_calls"]:
            send_message({
                "id": req_id,
                "tool_calls": delta["tool_calls"],
                "chunk": "",
                "done": False,
            })

    send_done(req_id)


def handle_ollama(req_id, params):
    """使用 Ollama HTTP API"""
    import requests
    import json

    base_url = params.get("ollama_url", "http://127.0.0.1:11434")
    model = params.get("model", "gemma4:12b")
    messages = params.get("messages", [])
    temperature = params.get("temperature", 0.7)
    max_tokens = params.get("max_tokens", 2048)

    tools = params.get("tools", [])

    body = {
        "model": model,
        "messages": messages,
        "stream": True,
        "options": {
            "temperature": temperature,
            "num_predict": max_tokens,
        },
    }
    if tools:
        body["tools"] = tools

    resp = requests.post(f"{base_url}/api/chat", json=body, stream=True)
    for line in resp.iter_lines():
        if not line:
            continue
        data = json.loads(line)
        msg = data.get("message", {})

        if msg.get("tool_calls"):
            send_message({
                "id": req_id,
                "tool_calls": msg["tool_calls"],
                "chunk": "",
                "done": False,
            })

        content = msg.get("content", "")
        if content:
            send_chunk(req_id, content)

        if data.get("done"):
            break

    send_done(req_id)


def handle(req_id, method, params):
    if method == "shutdown":
        sys.exit(0)
    elif method == "chat":
        if BACKEND == "ollama":
            handle_ollama(req_id, params)
        else:
            handle_llama_cpp(req_id, params)
    elif method == "embed":
        embed(req_id, params)
    else:
        send_message({"id": req_id, "result": None, "error": f"Unknown: {method}"})


def embed(req_id, params):
    """Embedding 推理"""
    texts = params.get("texts", [])
    if not texts:
        send_message({"id": req_id, "result": [], "error": None})
        return

    try:
        from sentence_transformers import SentenceTransformer
        model = SentenceTransformer(params.get("model_name", "BAAI/bge-m3"))
        vecs = model.encode(texts, normalize_embeddings=True).tolist()
        send_message({"id": req_id, "result": vecs, "error": None})
    except ImportError:
        send_message({"id": req_id, "result": None, "error": "sentence-transformers not installed"})


if __name__ == "__main__":
    from protocol import handle_requests
    handle_requests(handle)
