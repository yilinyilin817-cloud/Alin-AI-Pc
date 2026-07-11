use anyhow::Result;
use std::collections::HashMap;

/// 简易摘要器（存储并查重）
pub struct Summarizer {
    store: HashMap<String, String>,
}

impl Summarizer {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    /// 存储摘要
    pub fn store(&mut self, session_id: &str, summary: &str) {
        self.store.insert(session_id.to_string(), summary.to_string());
    }

    /// 获取摘要
    pub fn get(&self, session_id: &str) -> Option<&str> {
        self.store.get(session_id).map(|s| s.as_str())
    }
}
