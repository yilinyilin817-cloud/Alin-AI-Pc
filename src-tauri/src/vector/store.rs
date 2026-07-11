use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 向量命中结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorHit {
    pub id: String,
    pub distance: f32,
    pub payload: serde_json::Value,
}

/// 向量存储统一抽象
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// 创建集合（指定维度）
    async fn create_collection(&self, name: &str, dimension: usize) -> anyhow::Result<()>;
    /// 插入/更新向量
    async fn upsert(&self, collection: &str, id: &str, vector: Vec<f32>, payload: serde_json::Value) -> anyhow::Result<()>;
    /// 搜索最近邻
    async fn search(&self, collection: &str, query: &[f32], k: usize) -> anyhow::Result<Vec<VectorHit>>;
    /// 删除向量
    async fn delete(&self, collection: &str, id: &str) -> anyhow::Result<()>;
    /// 集合是否存在
    async fn collection_exists(&self, name: &str) -> anyhow::Result<bool>;
}
