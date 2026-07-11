use tauri::State;
use crate::state::AppState;

#[tauri::command]
pub fn list_knowledge_bases(state: State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    state.db.with_conn(|conn| {
        let mut stmt = conn.prepare("SELECT id, name, description, created_at FROM knowledge_base ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "description": row.get::<_, Option<String>>(2)?,
                "createdAt": row.get::<_, String>(3)?,
            }))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
    }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_knowledge_base(state: State<'_, AppState>, name: String, description: Option<String>) -> Result<serde_json::Value, String> {
    let id = format!("kb_{}", uuid::Uuid::new_v4());
    state.db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO knowledge_base (id, name, description) VALUES (?1, ?2, ?3)",
            rusqlite::params![id, name, description],
        )?;
        Ok(serde_json::json!({"id": id, "name": name, "description": description}))
    }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_knowledge_docs(state: State<'_, AppState>, kb_id: String) -> Result<Vec<serde_json::Value>, String> {
    state.db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, title, source, chunk_type, chunk_count, created_at FROM knowledge_doc WHERE kb_id = ?1 ORDER BY created_at"
        )?;
        let rows = stmt.query_map(rusqlite::params![kb_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "title": row.get::<_, Option<String>>(1)?,
                "source": row.get::<_, Option<String>>(2)?,
                "chunkType": row.get::<_, String>(3)?,
                "chunkCount": row.get::<_, i64>(4)?,
                "createdAt": row.get::<_, String>(5)?,
            }))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
    }).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_document(
    state: State<'_, AppState>,
    kb_id: String,
    title: String,
    source: String,
    content: String,
    chunk_type: Option<String>,
) -> Result<String, String> {
    let ct = chunk_type.unwrap_or_else(|| "text".to_string());
    crate::rag::indexer::import_document(
        &state.db,
        &*state.vector_store,
        &kb_id,
        &title,
        &source,
        &content,
        &ct,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_doc(state: State<'_, AppState>, doc_id: String) -> Result<(), String> {
    crate::rag::indexer::delete_document(&state.db, &*state.vector_store, &doc_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_knowledge(
    state: State<'_, AppState>,
    query: String,
    kb_id: Option<String>,
    top_k: Option<usize>,
) -> Result<Vec<serde_json::Value>, String> {
    let results = crate::rag::retriever::search(
        &state.db,
        &*state.vector_store,
        &query,
        kb_id.as_deref(),
        top_k.unwrap_or(5),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(results.into_iter().map(|r| serde_json::json!({
        "chunkId": r.chunk_id,
        "text": r.text,
        "score": r.score,
        "docTitle": r.doc_title,
    })).collect())
}
