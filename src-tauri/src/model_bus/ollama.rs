use crate::model_bus::provider::*;
use base64::Engine as _;
use futures_util::stream::{self, BoxStream};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Ollama Provider（HTTP 到本地 Ollama 服务）
#[derive(Debug)]
pub struct OllamaProvider {
    base_url: String,
    model: String,
    client: Arc<Client>,
    caps: Capabilities,
    available: Arc<Mutex<Option<bool>>>,
}

impl OllamaProvider {
    pub fn new(model: &str) -> Self {
        Self {
            base_url: "http://127.0.0.1:11434".into(),
            model: model.to_string(),
            client: Arc::new(Client::new()),
            caps: Capabilities {
                vision: true,  // Gemma 4 12B / Qwen3-VL
                audio_input: false, // Ollama API 暂无音频
                video: false,
                tool_use: true,
                max_context: 131072,
            },
            available: Arc::new(Mutex::new(None)),
        }
    }

    /// 检查 Ollama 是否可用
    pub async fn is_available(&self) -> bool {
        let mut cache = self.available.lock().await;
        if let Some(val) = *cache {
            return val;
        }
        let ok = self
            .client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .is_ok();
        *cache = Some(ok);
        ok
    }

    /// 设置可用性（用于外部 ping）
    pub async fn set_availability(&self, available: bool) {
        let mut cache = self.available.lock().await;
        *cache = Some(available);
    }
}

impl ModelProvider for OllamaProvider {
    fn id(&self) -> &str {
        &self.model
    }

    fn capabilities(&self) -> Capabilities {
        self.caps.clone()
    }

    fn chat_stream(&self, req: ChatRequest) -> BoxStream<'static, ChatChunk> {
        let client = self.client.clone();
        let url = format!("{}/api/chat", self.base_url);
        let model = self.model.clone();
        let available = self.available.clone();

        // 构造 Ollama 请求体
        let mut ollama_messages = Vec::new();
        for msg in &req.messages {
            let content = build_ollama_content(&msg.content);
            let mut message_obj = serde_json::json!({
                "role": msg.role,
                "content": content,
            });

            // 图像附件
            let images = collect_images(&msg.content);
            if !images.is_empty() {
                message_obj["images"] = serde_json::Value::Array(images);
            }

            // 工具调用
            if let Some(ref tool_calls) = msg.tool_calls {
                let calls: Vec<Value> = tool_calls
                    .iter()
                    .map(|tc| {
                        serde_json::json!({
                            "function": {
                                "name": tc.name,
                                "arguments": tc.arguments,
                            }
                        })
                    })
                    .collect();
                if !calls.is_empty() {
                    message_obj["tool_calls"] = serde_json::Value::Array(calls);
                }
            }

            ollama_messages.push(message_obj);
        }

        // 工具注入（Ollama 支持 tools）
        let tools: Vec<Value> = req
            .tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect();

        let body = serde_json::json!({
            "model": model,
            "messages": ollama_messages,
            "stream": true,
            "options": {
                "temperature": req.temperature,
                "top_p": req.top_p,
                "num_predict": req.max_tokens,
            },
            "tools": tools,
        });

        Box::pin(async_stream::stream! {
            // 检查可用性
            let avail = *available.lock().await;
            if avail == Some(false) {
                yield ChatChunk { content: String::new(), tool_calls: None, done: true };
                return;
            }

            let response = client.post(&url).json(&body).send().await;
            let response = match response {
                Ok(r) => r,
                Err(e) => {
                    log::warn!("Ollama request failed: {e}");
                    *available.lock().await = Some(false);
                    yield ChatChunk { content: "(Ollama 未启动，回退 mock)".into(), tool_calls: None, done: false };
                    yield ChatChunk { content: String::new(), tool_calls: None, done: true };
                    return;
                }
            };

            let mut stream = response.bytes_stream();
            let mut buf = String::new();

            while let Some(chunk) = stream.next().await {
                let chunk = match chunk {
                    Ok(b) => b,
                    Err(_) => break,
                };
                buf.push_str(&String::from_utf8_lossy(&chunk));
                while let Some(pos) = buf.find('\n') {
                    let line: String = buf.drain(..=pos).collect();
                    let line = line.trim();
                    if line.is_empty() { continue; }

                    // 解析 ndjson
                    if let Ok(ollama_resp) = serde_json::from_str::<OllamaChatResponse>(line) {
                        // 检查工具调用
                        if let Some(tool_calls) = ollama_resp.message.as_ref().and_then(|m| m.tool_calls.clone()) {
                            let calls: Vec<ToolCall> = tool_calls.into_iter().filter_map(|t| {
                                let f = t.function?;
                                let name = f.name?;
                                let args = f.arguments?;
                                Some(ToolCall {
                                    id: format!("tc_{}", uuid::Uuid::new_v4()),
                                    name,
                                    arguments: args,
                                })
                            }).collect();
                            if !calls.is_empty() {
                                yield ChatChunk { content: String::new(), tool_calls: Some(calls), done: false };
                            }
                        }

                        // 检查文本内容
                        if let Some(content) = ollama_resp.message.as_ref().and_then(|m| m.content.as_ref()) {
                            if !content.is_empty() {
                                yield ChatChunk { content: content.clone(), tool_calls: None, done: false };
                            }
                        }

                        if ollama_resp.done.unwrap_or(false) {
                            break;
                        }
                    }
                }
            }
            yield ChatChunk { content: String::new(), tool_calls: None, done: true };
        })
    }

    fn embed(&self, texts: &[String]) -> BoxStream<'static, Vec<Vec<f32>>> {
        let client = self.client.clone();
        let url = format!("{}/api/embed", self.base_url);
        let model = self.model.clone();
        let texts = texts.to_vec();

        Box::pin(async_stream::stream! {
            let body = serde_json::json!({
                "model": model,
                "input": texts,
            });
            let resp = client.post(&url).json(&body).send().await;
            match resp {
                Ok(r) => {
                    if let Ok(data) = r.json::<OllamaEmbedResponse>().await {
                        yield data.embeddings;
                        return;
                    }
                }
                Err(e) => log::warn!("Ollama embed failed: {e}"),
            }
            yield vec![];
        })
    }
}

fn build_ollama_content(parts: &[ContentPart]) -> String {
    let mut text = String::new();
    for part in parts {
        match part {
            ContentPart::Text(t) => text.push_str(t),
            _ => {} // 图像通过 Ollama 'images' 字段传
        }
    }
    text
}

fn collect_images(parts: &[ContentPart]) -> Vec<Value> {
    let mut images = Vec::new();
    for part in parts {
        match part {
            ContentPart::ImageBytes(data) => {
                images.push(Value::String(base64::engine::GeneralPurpose::new(
                    &base64::alphabet::STANDARD,
                    base64::engine::general_purpose::PAD,
                ).encode(data)));
            }
            ContentPart::ImageUrl(url) => {
                // 尝试将本地/远程 URL 读成 base64（简单实现：仅支持 data URL）
                if let Some(b64) = url.strip_prefix("data:").and_then(|s| s.split_once(',')).map(|(_, b64)| b64) {
                    images.push(Value::String(b64.to_string()));
                }
            }
            _ => {}
        }
    }
    images
}

// Ollama API 响应格式
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OllamaChatResponse {
    model: Option<String>,
    message: Option<OllamaMessage>,
    done: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OllamaMessage {
    role: Option<String>,
    content: Option<String>,
    tool_calls: Option<Vec<OllamaToolCall>>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct OllamaToolCall {
    function: Option<OllamaFunction>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct OllamaFunction {
    name: Option<String>,
    arguments: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OllamaEmbedResponse {
    embeddings: Vec<Vec<f32>>,
}
