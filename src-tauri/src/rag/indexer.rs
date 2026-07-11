use crate::storage::Database;
use crate::vector::store::VectorStore;
use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

/// 文档切分结果
pub struct Chunk {
    pub text: String,
    pub seq: usize,
}

/// 文本切分（简易滑动窗口）
fn split_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<Chunk> {
    if text.len() <= chunk_size {
        return vec![Chunk { text: text.to_string(), seq: 0 }];
    }
    let mut chunks = Vec::new();
    let mut start = 0;
    let mut seq = 0;
    while start < text.len() {
        let end = (start + chunk_size).min(text.len());
        // 尽量在句号处分句
        let mut cut = end;
        if end < text.len() {
            if let Some(pos) = text[start..end].rfind(|c: char| c == '.' || c == '!' || c == '？' || c == '。' || c == '！') {
                cut = start + pos + 1;
            }
        }
        chunks.push(Chunk {
            text: text[start..cut].to_string(),
            seq,
        });
        seq += 1;
        start = if cut > overlap { cut - overlap } else { cut };
    }
    chunks
}

/// 导入文档到知识库
pub async fn import_document(
    db: &Database,
    vector: &dyn VectorStore,
    kb_id: &str,
    title: &str,
    source: &str,
    content: &str,
    chunk_type: &str,
) -> Result<String> {
    let doc_id = format!("doc_{}", Uuid::new_v4());

    // 分块
    let chunks = if chunk_type == "image" {
        vec![Chunk { text: content.to_string(), seq: 0 }]
    } else {
        split_text(content, 512, 50)
    };

    // 存入 knowledge_doc
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO knowledge_doc (id, kb_id, title, source, chunk_type, chunk_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![doc_id, kb_id, title, source, chunk_type, chunks.len() as i64],
        )?;
        Ok(())
    })?;

    // 存入 knowledge_chunk + 向量化
    for chunk in &chunks {
        let chunk_id = format!("{}_{}", doc_id, chunk.seq);

        // 用 bge-m3 生成向量（通过 LLM embed fallback）
        let embedding_vec = vec![0.0f32; 1024]; // 占位，真实由 embedding worker 填充

        // 插入 knowledge_chunk
                        db.with_conn(|conn| {
                            conn.execute(
                                "INSERT INTO knowledge_chunk (id, doc_id, seq, text) VALUES (?1, ?2, ?3, ?4)",
                                rusqlite::params![chunk_id, doc_id, chunk.seq, chunk.text],
                            )?;
                            Ok(())
                        }).map_err(|e: rusqlite::Error| anyhow::anyhow!("{e}"))?;

        // 向量化插入
        vector
            .upsert("knowledge", &chunk_id, embedding_vec.clone(), serde_json::json!({
                "doc_id": doc_id,
                "kb_id": kb_id,
                "seq": chunk.seq,
                "text": chunk.text,
            }))
            .await?;
    }

    Ok(doc_id)
}

/// 删除文档
pub async fn delete_document(db: &Database, vector: &dyn VectorStore, doc_id: &str) -> Result<()> {
    let chunks: Vec<String> = db.with_conn(|conn| {
        let mut stmt = conn.prepare("SELECT id FROM knowledge_chunk WHERE doc_id = ?1")?;
        let rows = stmt.query_map(rusqlite::params![doc_id], |row| row.get(0))?;
        rows.collect::<Result<Vec<_>, _>>()
    })?;

    for chunk_id in &chunks {
        vector.delete("knowledge", chunk_id).await?;
    }

    db.with_conn(|conn| {
        conn.execute("DELETE FROM knowledge_doc WHERE id = ?1", rusqlite::params![doc_id])?;
        Ok(())
    })?;

    Ok(())
}
