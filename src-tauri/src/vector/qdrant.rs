use crate::vector::store::{VectorHit, VectorStore};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Qdrant 服务器实现
pub struct QdrantStore {
    client: Arc<Client>,
    base_url: String,
}

impl QdrantStore {
    pub fn new(base_url: &str) -> Self {
        let url = if base_url.is_empty() {
            "http://127.0.0.1:6333".to_string()
        } else {
            base_url.to_string()
        };
        Self {
            client: Arc::new(Client::new()),
            base_url: url,
        }
    }
}

#[async_trait]
impl VectorStore for QdrantStore {
    async fn create_collection(&self, name: &str, dimension: usize) -> Result<()> {
        let body = serde_json::json!({
            "name": name,
            "vectors": { "size": dimension, "distance": "Cosine" }
        });
        self.client
            .put(format!("{}/collections/{}", self.base_url, name))
            .json(&body)
            .send()
            .await?;
        Ok(())
    }

    async fn upsert(&self, collection: &str, id: &str, vector: Vec<f32>, payload: serde_json::Value) -> Result<()> {
        let body = serde_json::json!({
            "points": [{
                "id": id,
                "vector": vector,
                "payload": payload,
            }]
        });
        self.client
            .put(format!("{}/collections/{}/points", self.base_url, collection))
            .json(&body)
            .send()
            .await?;
        Ok(())
    }

    async fn search(&self, collection: &str, query: &[f32], k: usize) -> Result<Vec<VectorHit>> {
        let body = serde_json::json!({
            "vector": query,
            "limit": k,
            "with_payload": true,
        });
        let resp = self
            .client
            .post(format!("{}/collections/{}/points/search", self.base_url, collection))
            .json(&body)
            .send()
            .await?;

        let data: QdrantSearchResp = resp.json().await?;
        let hits = data
            .result
            .unwrap_or_default()
            .into_iter()
            .map(|p| VectorHit {
                id: format!("{:?}", p.id),
                distance: p.score.unwrap_or(0.0),
                payload: p.payload.unwrap_or(serde_json::Value::Null),
            })
            .collect();
        Ok(hits)
    }

    async fn delete(&self, collection: &str, id: &str) -> Result<()> {
        let body = serde_json::json!({ "points": [id] });
        self.client
            .post(format!("{}/collections/{}/points/delete", self.base_url, collection))
            .json(&body)
            .send()
            .await?;
        Ok(())
    }

    async fn collection_exists(&self, name: &str) -> Result<bool> {
        let resp = self
            .client
            .get(format!("{}/collections/{}", self.base_url, name))
            .send()
            .await?;
        Ok(resp.status().is_success())
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct QdrantSearchResp {
    result: Option<Vec<QdrantPoint>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct QdrantPoint {
    id: serde_json::Value,
    score: Option<f32>,
    payload: Option<serde_json::Value>,
}
