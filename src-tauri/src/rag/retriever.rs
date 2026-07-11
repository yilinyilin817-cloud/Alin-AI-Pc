use crate::storage::Database;
use crate::vector::store::VectorStore;
use anyhow::Result;
use serde::Serialize;

/// 检索结果
#[derive(Debug, Clone, Serialize)]
pub struct RetrievedChunk {
    pub chunk_id: String,
    pub text: String,
    pub score: f32,
    pub doc_title: Option<String>,
}

/// 混合检索（向量 + FTS5 关键词）
pub async fn search(
    db: &Database,
    vector: &dyn VectorStore,
    query: &str,
    kb_id: Option<&str>,
    top_k: usize,
) -> Result<Vec<RetrievedChunk>> {
    // 1. 向量召回（使用占位向量，实际由 embedding worker 实时编码）
    let query_vec = vec![0.0f32; 1024];
    let vector_hits = vector.search("knowledge", &query_vec, top_k).await?;

    // 2. FTS5 关键词召回
    let fts_hits = keyword_search(db, query, kb_id, top_k)?;

    // 3. RRF 融合
    let mut scored: std::collections::HashMap<String, (f32, String, Option<String>)> = std::collections::HashMap::new();

    for (rank, hit) in vector_hits.iter().enumerate() {
        let rrf_score = 1.0 / (rank as f32 + 61.0);
        let text = hit.payload.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let doc_title = hit.payload.get("doc_title").and_then(|v| v.as_str()).map(|s| s.to_string());
        scored.insert(hit.id.clone(), (rrf_score, text, doc_title));
    }

    for (rank, hit) in fts_hits.iter().enumerate() {
        let rrf_score = 1.0 / (rank as f32 + 61.0);
        scored
            .entry(hit.chunk_id.clone())
            .and_modify(|(s, _, _)| *s += rrf_score)
            .or_insert((rrf_score, hit.text.clone(), hit.doc_title.clone()));
    }

    let mut results: Vec<RetrievedChunk> = scored
        .into_iter()
        .map(|(chunk_id, (score, text, doc_title))| RetrievedChunk {
            chunk_id,
            text,
            score,
            doc_title,
        })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(top_k);
    Ok(results)
}

/// FTS5 关键词检索
fn keyword_search(
    db: &Database,
    query: &str,
    kb_id: Option<&str>,
    top_k: usize,
) -> Result<Vec<RetrievedChunk>> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }

    // 构建 FTS5 查询词
    let fts_query = query
        .split_whitespace()
        .filter(|w| w.len() > 1)
        .map(|w| format!("\"{}\"", w))
        .collect::<Vec<_>>()
        .join(" OR ");

    if fts_query.is_empty() {
        return Ok(vec![]);
    }

    db.with_conn(|conn| {
        let sql = if kb_id.is_some() {
            "SELECT c.id, c.text, d.title FROM knowledge_chunk_fts fts
             JOIN knowledge_chunk c ON fts.rowid = c.rowid
             JOIN knowledge_doc d ON c.doc_id = d.id
             WHERE knowledge_chunk_fts MATCH ?1 AND d.kb_id = ?2
             ORDER BY rank LIMIT ?3"
        } else {
            "SELECT c.id, c.text, d.title FROM knowledge_chunk_fts fts
             JOIN knowledge_chunk c ON fts.rowid = c.rowid
             JOIN knowledge_doc d ON c.doc_id = d.id
             WHERE knowledge_chunk_fts MATCH ?1
             ORDER BY rank LIMIT ?2"
        };

        let mut stmt = conn.prepare(sql)?;

        let rows: Vec<RetrievedChunk> = if let Some(kb) = kb_id {
            let r = stmt.query_map(
                rusqlite::params![fts_query, kb, top_k as i64],
                |row| {
                    Ok(RetrievedChunk {
                        chunk_id: row.get(0)?,
                        text: row.get(1)?,
                        score: 0.5,
                        doc_title: row.get(2)?,
                    })
                },
            )?;
            r.collect::<Result<Vec<_>, _>>()?
        } else {
            let r = stmt.query_map(
                rusqlite::params![fts_query, top_k as i64],
                |row| {
                    Ok(RetrievedChunk {
                        chunk_id: row.get(0)?,
                        text: row.get(1)?,
                        score: 0.5,
                        doc_title: row.get(2)?,
                    })
                },
            )?;
            r.collect::<Result<Vec<_>, _>>()?
        };

            Ok(rows)
        }).map_err(|e| anyhow::anyhow!("keyword search: {e}"))
    }
