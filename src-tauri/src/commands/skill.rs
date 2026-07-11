use tauri::State;
use crate::state::AppState;
use crate::skill::registry::SkillRegistry;

#[tauri::command]
pub fn list_skills(state: State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    state.db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, enabled, config_json FROM skill ORDER BY name"
        )?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let desc: Option<String> = row.get(2)?;
            let enabled: bool = row.get(3)?;
            Ok(serde_json::json!({
                "id": id,
                "name": name,
                "description": desc,
                "enabled": enabled,
                "icon": "Bolt",
                "permissions": [],
                "approvalMode": "once"
            }))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
    }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_skill(state: State<'_, AppState>, skill_name: String, enabled: bool) -> Result<(), String> {
    state.db.with_conn(|conn| {
        conn.execute(
            "UPDATE skill SET enabled = ?1 WHERE name = ?2",
            rusqlite::params![enabled, skill_name],
        )?;
        Ok(())
    }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn approve_skill_permission(state: State<'_, AppState>, skill_name: String, status: String) -> Result<(), String> {
    let id = format!("sp_{}", uuid::Uuid::new_v4());
    state.db.with_conn(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO skill_permission (id, skill_name, status, approved_at) VALUES (?1, ?2, ?3, datetime('now'))",
            rusqlite::params![id, skill_name, status],
        )?;
        Ok(())
    }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_tool_call_logs(state: State<'_, AppState>, session_id: Option<String>, limit: Option<i64>) -> Result<Vec<serde_json::Value>, String> {
    let limit = limit.unwrap_or(50);
    state.db.with_conn(|conn| {
        let sql = if let Some(ref sid) = session_id {
            format!("SELECT id, session_id, skill_name, args_json, result_json, status, duration_ms, created_at FROM tool_call_log WHERE session_id = '{}' ORDER BY created_at DESC LIMIT {}", sid, limit)
        } else {
            format!("SELECT id, session_id, skill_name, args_json, result_json, status, duration_ms, created_at FROM tool_call_log ORDER BY created_at DESC LIMIT {}", limit)
        };
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "sessionId": row.get::<_, String>(1)?,
                "skillName": row.get::<_, String>(2)?,
                "argsJson": row.get::<_, Option<String>>(3)?,
                "resultJson": row.get::<_, Option<String>>(4)?,
                "status": row.get::<_, String>(5)?,
                "durationMs": row.get::<_, i64>(6)?,
                "createdAt": row.get::<_, String>(7)?,
            }))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
    }).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_skill_manual(
    state: State<'_, AppState>,
    session_id: String,
    skill_name: String,
    args: serde_json::Value,
) -> Result<String, String> {
    // 懒构造 SkillRegistry，同时加载 DB 技能与插件技能
    let mut registry = SkillRegistry::new();
    {
        let plugin_registry = state
            .plugin_registry
            .read()
            .map_err(|e| format!("锁定插件注册表失败: {e}"))?;
        registry
            .load_all(&state.db, &*plugin_registry)
            .map_err(|e| e.to_string())?;
    }

    crate::skill::executor::execute_skill(&registry, &state.db, &session_id, &skill_name, args)
        .await
        .map_err(|e| e.to_string())
}
