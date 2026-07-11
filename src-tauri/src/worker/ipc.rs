use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

static REQ_ID: AtomicU64 = AtomicU64::new(0);

pub fn next_id() -> u64 {
    REQ_ID.fetch_add(1, Ordering::Relaxed)
}

/// Worker 请求（Rust → Python）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRequest {
    pub id: u64,
    pub method: String,
    pub params: serde_json::Value,
}

/// Worker 响应（Python → Rust）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResponse {
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk: Option<String>,
    pub done: bool,
}

/// 构建 Worker 请求体（MessagePack 帧）
pub fn build_request(method: &str, params: serde_json::Value) -> WorkerRequest {
    WorkerRequest {
        id: next_id(),
        method: method.into(),
        params,
    }
}
