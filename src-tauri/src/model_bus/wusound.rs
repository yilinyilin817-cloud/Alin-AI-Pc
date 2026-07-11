use crate::model_bus::provider::*;
use futures_util::stream::{self, BoxStream};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// 悟声 TTS Provider
#[derive(Debug)]
pub struct WusoundProvider {
    provider_id: String,
    name: String,
    base_url: String,
    api_key: String,
    client: Arc<Client>,
    available: Arc<Mutex<Option<bool>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WusoundVoice {
    pub id: String,
    pub name: String,
    pub status: String,
    pub metadata: Option<WusoundVoiceMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WusoundVoiceMetadata {
    pub avatar: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub language: Option<Vec<String>>,
    #[serde(default)]
    pub gender: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    pub prompts: Option<Vec<WusoundPrompt>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WusoundPrompt {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// 悟声 TTS 合成参数
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WusoundSynthesizeOptions {
    /// 语速倍率，1.0 为正常
    #[serde(default)]
    pub speed: Option<f32>,
    /// 音调倍率，1.0 为正常
    #[serde(default)]
    pub pitch: Option<f32>,
    /// 音量倍率，1.0 为正常
    #[serde(default)]
    pub volume: Option<f32>,
    /// 音频格式：wav / mp3 / opus / pcm
    #[serde(default)]
    pub format: Option<String>,
    /// 采样率：8000 / 16000 / 24000 / 48000
    #[serde(default)]
    pub sample_rate: Option<u32>,
}

/// 悟声账户/余额信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WusoundQuota {
    /// 已用字符数
    pub used_chars: u64,
    /// 总额度
    pub total_chars: u64,
    /// 剩余字符数
    pub remaining_chars: u64,
    /// 套餐等级（自由/标准/企业等，API 自定义）
    #[serde(default)]
    pub tier: Option<String>,
    /// 原始负载，便于前端调试
    #[serde(default)]
    pub raw: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct WusoundResponse<T> {
    status: i32,
    message: String,
    data: Option<T>,
}

impl WusoundProvider {
    pub fn new(provider_id: &str, name: &str, base_url: &str, api_key: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            provider_id: provider_id.to_string(),
            name: name.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            client: Arc::new(client),
            available: Arc::new(Mutex::new(None)),
        }
    }

    /// 验证 API 连通性并获取语音角色列表
    pub async fn verify(&self) -> Result<Vec<WusoundVoice>, String> {
        let url = format!("{}/voice", self.base_url);
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

        let body: WusoundResponse<Vec<WusoundVoice>> = resp
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {e}"))?;

        if body.status != 200 {
            return Err(format!("API 错误: {}", body.message));
        }

        Ok(body.data.unwrap_or_default())
    }

    /// 查询账户/配额信息（可选接口，若后端不支持则返回 None）
    pub async fn quota(&self) -> Result<Option<WusoundQuota>, String> {
        let url = format!("{}/quota", self.base_url);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| format!("连接失败: {e}"))?;

        if !resp.status().is_success() {
            return Ok(None);
        }

        // 解析为通用 JSON，再尝试映射
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {e}"))?;

        // 兼容结构：{ status, data: { used, total, ... } }
        let used = raw
            .get("data")
            .and_then(|d| d.get("used_chars").or_else(|| d.get("used")))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let total = raw
            .get("data")
            .and_then(|d| d.get("total_chars").or_else(|| d.get("total")))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let tier = raw
            .get("data")
            .and_then(|d| d.get("tier").or_else(|| d.get("plan")))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let remaining = total.saturating_sub(used);
        Ok(Some(WusoundQuota {
            used_chars: used,
            total_chars: total,
            remaining_chars: remaining,
            tier,
            raw: Some(raw),
        }))
    }

    /// TTS 合成（同步），支持可选的语速/音调/格式参数
    pub async fn synthesize(
        &self,
        text: &str,
        voice_id: &str,
        prompt_id: Option<&str>,
        options: Option<WusoundSynthesizeOptions>,
    ) -> Result<Vec<u8>, String> {
        let url = format!("{}/tts/sync", self.base_url);

        let mut body = serde_json::json!({
            "text": text,
            "voice_id": voice_id,
        });

        if let Some(pid) = prompt_id {
            body["prompt_id"] = serde_json::Value::String(pid.to_string());
        }

        if let Some(opts) = options {
            if let Some(s) = opts.speed {
                body["speed"] = serde_json::json!(s);
            }
            if let Some(p) = opts.pitch {
                body["pitch"] = serde_json::json!(p);
            }
            if let Some(v) = opts.volume {
                body["volume"] = serde_json::json!(v);
            }
            if let Some(f) = opts.format {
                body["format"] = serde_json::Value::String(f);
            }
            if let Some(sr) = opts.sample_rate {
                body["sample_rate"] = serde_json::json!(sr);
            }
        }

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("TTS 请求失败: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("TTS API 返回错误状态: {status} {text}"));
        }

        // 根据 Content-Type 判断是否真的是音频
        let ct = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        if !ct.is_empty()
            && !ct.starts_with("audio/")
            && !ct.starts_with("application/octet-stream")
        {
            // 可能是 JSON 错误体
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("TTS 返回非音频内容: {ct} {text}"));
        }

        let bytes = resp.bytes().await.map_err(|e| format!("读取音频失败: {e}"))?;
        Ok(bytes.to_vec())
    }

    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl ModelProvider for WusoundProvider {
    fn id(&self) -> &str {
        &self.provider_id
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            vision: false,
            audio_input: false,
            video: false,
            tool_use: false,
            max_context: 0,
        }
    }

    fn chat_stream(&self, _req: ChatRequest) -> BoxStream<'static, ChatChunk> {
        // 悟声是 TTS 服务，不支持聊天
        Box::pin(stream::once(async {
            ChatChunk {
                content: "悟声是 TTS 服务商，不支持聊天功能".to_string(),
                tool_calls: None,
                done: true,
            }
        }))
    }

    fn embed(&self, _texts: &[String]) -> BoxStream<'static, Vec<Vec<f32>>> {
        // 悟声不支持嵌入
        Box::pin(stream::once(async {
            log::warn!("Wusound provider does not support embedding");
            vec![]
        }))
    }
}
