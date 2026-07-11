use crate::model_bus::provider::*;
use futures_util::stream::{self, BoxStream};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 云服务商 Provider（OpenAI 兼容 API）
#[derive(Debug)]
pub struct CloudProvider {
    provider_id: String,
    name: String,
    base_url: String,
    api_key: String,
    model: String,
    client: Arc<Client>,
    caps: Capabilities,
    available: Arc<Mutex<Option<bool>>>,
}

impl CloudProvider {
    pub fn new(provider_id: &str, name: &str, base_url: &str, api_key: &str, model: &str) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            name: name.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            client: Arc::new(Client::new()),
            caps: Capabilities {
                vision: false,
                audio_input: false,
                video: false,
                tool_use: true,
                max_context: 128000,
            },
            available: Arc::new(Mutex::new(None)),
        }
    }

    /// 验证 API 连通性
    pub async fn verify(&self) -> Result<Vec<CloudModel>, String> {
        let url = format!("{}/models", self.base_url);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| format!("连接失败: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("API 返回错误状态: {}", resp.status()));
        }

        let body: Value = resp
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {e}"))?;

        let models = body["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| {
                        let id = m["id"].as_str()?.to_string();
                        Some(CloudModel { id })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(models)
    }

    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudModel {
    pub id: String,
}

impl ModelProvider for CloudProvider {
    fn id(&self) -> &str {
        &self.model
    }

    fn capabilities(&self) -> Capabilities {
        self.caps.clone()
    }

    fn chat_stream(&self, req: ChatRequest) -> BoxStream<'static, ChatChunk> {
        let client = self.client.clone();
        let url = format!("{}/chat/completions", self.base_url);
        let api_key = self.api_key.clone();
        let model = self.model.clone();
        let available = self.available.clone();

        // 构造 OpenAI 兼容请求
        let messages: Vec<Value> = req
            .messages
            .iter()
            .map(|msg| {
                let text = msg
                    .content
                    .iter()
                    .filter_map(|p| match p {
                        ContentPart::Text(t) => Some(t.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");
                serde_json::json!({
                    "role": msg.role,
                    "content": text,
                })
            })
            .collect();

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
            "messages": messages,
            "stream": true,
            "temperature": req.temperature,
            "top_p": req.top_p,
            "max_tokens": req.max_tokens,
            "tools": tools,
        });

        Box::pin(async_stream::stream! {
            let resp = client
                .post(&url)
                .bearer_auth(&api_key)
                .json(&body)
                .send()
                .await;

            let resp = match resp {
                Ok(r) => r,
                Err(e) => {
                    log::warn!("Cloud provider request failed: {e}");
                    *available.lock().await = Some(false);
                    yield ChatChunk {
                        content: format!("(云服务商请求失败: {e})"),
                        tool_calls: None,
                        done: false,
                    };
                    yield ChatChunk { content: String::new(), tool_calls: None, done: true };
                    return;
                }
            };

            let mut stream = resp.bytes_stream();
            let mut buf = String::new();

            while let Some(chunk) = stream.next().await {
                let chunk = match chunk {
                    Ok(b) => b,
                    Err(_) => break,
                };
                buf.push_str(&String::from_utf8_lossy(&chunk));

                // 解析 SSE 格式
                while let Some(pos) = buf.find('\n') {
                    let line: String = buf.drain(..=pos).collect();
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            yield ChatChunk { content: String::new(), tool_calls: None, done: true };
                            return;
                        }

                        if let Ok(chunk_resp) = serde_json::from_str::<OpenAIChatChunk>(data) {
                            if let Some(choice) = chunk_resp.choices.first() {
                                // 检查工具调用
                                if let Some(tool_calls) = &choice.delta.tool_calls {
                                    let calls: Vec<ToolCall> = tool_calls
                                        .iter()
                                        .filter_map(|tc| {
                                            let func = tc.function.as_ref()?;
                                            let name = func.name.as_ref()?;
                                            let args: Value = func
                                                .arguments
                                                .as_ref()
                                                .and_then(|a| serde_json::from_str(a).ok())
                                                .unwrap_or(Value::Null);
                                            Some(ToolCall {
                                                id: tc.id.clone().unwrap_or_default(),
                                                name: name.clone(),
                                                arguments: args,
                                            })
                                        })
                                        .collect();
                                    if !calls.is_empty() {
                                        yield ChatChunk {
                                            content: String::new(),
                                            tool_calls: Some(calls),
                                            done: false,
                                        };
                                    }
                                }

                                // 文本内容
                                if let Some(content) = &choice.delta.content {
                                    if !content.is_empty() {
                                        yield ChatChunk {
                                            content: content.clone(),
                                            tool_calls: None,
                                            done: false,
                                        };
                                    }
                                }

                                if choice.finish_reason.is_some() {
                                    yield ChatChunk {
                                        content: String::new(),
                                        tool_calls: None,
                                        done: true,
                                    };
                                    return;
                                }
                            }
                        }
                    }
                }
            }
            yield ChatChunk { content: String::new(), tool_calls: None, done: true };
        })
    }

    fn embed(&self, _texts: &[String]) -> BoxStream<'static, Vec<Vec<f32>>> {
        Box::pin(stream::once(async {
            log::warn!("Cloud provider does not support embedding");
            vec![]
        }))
    }
}

// OpenAI API 响应格式
#[derive(Debug, Deserialize)]
struct OpenAIChatChunk {
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    delta: OpenAIDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIDelta {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAIToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OpenAIToolCall {
    id: Option<String>,
    function: Option<OpenAIFunction>,
}

#[derive(Debug, Deserialize)]
struct OpenAIFunction {
    name: Option<String>,
    arguments: Option<String>,
}
