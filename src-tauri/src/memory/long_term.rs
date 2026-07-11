use crate::storage::Database;
use crate::vector::store::VectorStore;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 长期记忆条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: String,
    pub persona_id: String,
    pub typ: String, // summary / event / preference
    pub content: String,
    pub importance: f64,
}

/// 保存记忆
pub async fn store(
    db: &Database,
    vector: &dyn VectorStore,
    persona_id: &str,
    typ: &str,
    content: &str,
    importance: f64,
) -> Result<String> {
    let id = format!("mem_{}", Uuid::new_v4());
    let vector_id = format!("{}_{id}", persona_id);

    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO memory (id, persona_id, type, content, embedding_id, importance)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, persona_id, typ, content, &vector_id, importance],
        )?;
            Ok(())
    })?;

    // 向量索引（占位向量）
    vector
        .upsert(
            &format!("memory_{persona_id}"),
            &vector_id,
            vec![0.0f32; 1024],
            serde_json::json!({
                "persona_id": persona_id,
                "type": typ,
                "content": content,
                "importance": importance,
            }),
        )
        .await?;

    Ok(id)
}

/// 召回记忆
pub async fn recall(
    db: &Database,
    vector: &dyn VectorStore,
    persona_id: &str,
    query: &str,
    top_k: usize,
) -> Result<Vec<MemoryItem>> {
    let _ = query; // 语义相似度召回用 query 向量；当前用占位向量
    let hits = vector
        .search(&format!("memory_{persona_id}"), &vec![0.0f32; 1024], top_k)
        .await?;

    let mut items = Vec::new();
    for hit in hits {
        let payload = &hit.payload;
        items.push(MemoryItem {
            id: hit.id,
            persona_id: payload.get("persona_id").and_then(|v| v.as_str()).unwrap_or(persona_id).to_string(),
            typ: payload.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            content: payload.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            importance: payload.get("importance").and_then(|v| v.as_f64()).unwrap_or(0.5),
        });
    }
    Ok(items)
}
