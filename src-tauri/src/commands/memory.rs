use tauri::State;
use crate::state::AppState;

#[tauri::command]
pub fn list_memories(state: State<'_, AppState>, persona_id: String) -> Result<Vec<serde_json::Value>, String> {
    state.db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, type, content, importance, created_at FROM memory WHERE persona_id = ?1 ORDER BY importance DESC LIMIT 50"
        )?;
        let rows = stmt.query_map(rusqlite::params![persona_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "type": row.get::<_, String>(1)?,
                "content": row.get::<_, String>(2)?,
                "importance": row.get::<_, f64>(3)?,
                "createdAt": row.get::<_, String>(4)?,
            }))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
    }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_memory(state: State<'_, AppState>, memory_id: String) -> Result<(), String> {
    state.db.with_conn(|conn| {
        conn.execute(
            "DELETE FROM memory WHERE id = ?1",
            rusqlite::params![memory_id],
        )?;
        Ok(())
    }).map_err(|e| e.to_string())
}
