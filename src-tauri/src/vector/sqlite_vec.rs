use crate::vector::store::{VectorHit, VectorStore};
use anyhow::{Context, Result};
use async_trait::async_trait;
use rusqlite::params;
use std::sync::Arc;

/// 使用 SQLite BLOB 存储 + Rust 全量余弦相似度计算的向量存储
/// 适用于桌面端中小规模（<10万条）向量检索
pub struct BlobVecStore {
    db: Arc<crate::storage::Database>,
}

impl BlobVecStore {
    pub fn new(db: Arc<crate::storage::Database>) -> Self {
        Self { db }
    }

    /// 初始化向量表（应用启动时调用一次）
    pub fn initialize(&self) -> Result<()> {
        self.db.with_conn(|conn| {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS vec_store (
                    collection TEXT NOT NULL,
                    id TEXT NOT NULL,
                    payload_json TEXT,
                    embedding BLOB,
                    created_at TEXT DEFAULT (datetime('now')),
                    PRIMARY KEY (collection, id)
                );
                CREATE INDEX IF NOT EXISTS idx_vec_collection ON vec_store(collection);
                "
            )?;
            Ok(())
        }).context("init vec store")
    }
}

/// 编码 Vec<f32> 为 BLOB（小端序 f32 bytes）
fn vec_to_blob(v: &[f32]) -> Vec<u8> {
    v.iter()
        .flat_map(|f| f.to_le_bytes())
        .collect()
}

/// 解码 BLOB 为 Vec<f32>
fn blob_to_vec(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// 余弦相似度
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

#[async_trait]
impl VectorStore for BlobVecStore {
    async fn create_collection(&self, _name: &str, _dimension: usize) -> Result<()> {
        // vec_store 表是全局的，collection 字段区分命名空间
        Ok(())
    }

    async fn upsert(&self, collection: &str, id: &str, vector: Vec<f32>, payload: serde_json::Value) -> Result<()> {
        let payload_str = serde_json::to_string(&payload).unwrap_or_default();
        let blob = vec_to_blob(&vector);
        let collection = collection.to_string();
        let id = id.to_string();

        self.db.with_conn(move |conn| {
            conn.execute(
                "INSERT OR REPLACE INTO vec_store (collection, id, payload_json, embedding) VALUES (?1, ?2, ?3, ?4)",
                params![&collection, &id, &payload_str, &blob],
            )?;
            Ok(())
        }).context("upsert vector")
    }

    async fn search(&self, collection: &str, query: &[f32], k: usize) -> Result<Vec<VectorHit>> {
        let collection = collection.to_string();
        let query_vec = query.to_vec();

        self.db.with_conn(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, payload_json, embedding FROM vec_store WHERE collection = ?1"
            )?;
            let rows = stmt.query_map(params![&collection], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            })?;

            let mut hits: Vec<VectorHit> = Vec::new();
            for row in rows {
                let (id, payload_str, blob) = row?;
                let stored_vec = blob_to_vec(&blob);
                let distance = 1.0 - cosine_similarity(&query_vec, &stored_vec); // 余弦距离
                let payload = payload_str
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or(serde_json::Value::Null);

                hits.push(VectorHit { id, distance, payload });
            }

            // 按距离升序排序（最近优先）
            hits.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
            hits.truncate(k);
            Ok(hits)
        }).context("search vectors")
    }

    async fn delete(&self, collection: &str, id: &str) -> Result<()> {
        let collection = collection.to_string();
        let id = id.to_string();
        self.db.with_conn(move |conn| {
            conn.execute(
                "DELETE FROM vec_store WHERE collection = ?1 AND id = ?2",
                params![&collection, &id],
            )?;
            Ok(())
        }).context("delete vector")
    }

    async fn collection_exists(&self, _name: &str) -> Result<bool> {
        // vec_store 表在 initialize 时创建
        Ok(true)
    }
}
