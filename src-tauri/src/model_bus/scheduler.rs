use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::model_bus::provider::ModelProvider;

/// 模型调度器（单例）
pub struct ModelScheduler {
    /// provider_id → Box<dyn ModelProvider>
    providers: RwLock<HashMap<String, Arc<dyn ModelProvider + Send + Sync>>>,
    /// 当前活跃的 provider_id
    active_id: RwLock<String>,
}

impl ModelScheduler {
    pub fn new(default_provider_id: &str) -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
            active_id: RwLock::new(default_provider_id.to_string()),
        }
    }

    /// 注册 Provider
    pub async fn register(&self, provider: Arc<dyn ModelProvider + Send + Sync>) {
        let id = provider.id().to_string();
        let mut providers = self.providers.write().await;
        providers.insert(id, provider);
    }

    /// 获取当前活跃 Provider
    pub async fn active(&self) -> Option<Arc<dyn ModelProvider + Send + Sync>> {
        let id = self.active_id.read().await;
        let providers = self.providers.read().await;
        providers.get(id.as_str()).cloned()
    }

    /// 切换活跃 Provider
    pub async fn set_active(&self, id: &str) {
        let mut active_id = self.active_id.write().await;
        *active_id = id.to_string();
    }

    /// 获取指定 Provider
    pub async fn get(&self, id: &str) -> Option<Arc<dyn ModelProvider + Send + Sync>> {
        let providers = self.providers.read().await;
        providers.get(id).cloned()
    }

    /// 列出所有注册的 Provider ID
    pub async fn list(&self) -> Vec<String> {
        let providers = self.providers.read().await;
        providers.keys().cloned().collect()
    }
}
