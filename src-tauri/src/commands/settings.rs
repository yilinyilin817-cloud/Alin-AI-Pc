use tauri::State;
use crate::state::AppState;

#[tauri::command]
pub fn load_settings(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    state.db.with_conn(|conn| {
        let mut stmt = conn.prepare("SELECT key, value_json FROM settings")?;
        let rows = stmt.query_map([], |row| {
            let key: String = row.get(0)?;
            let val: String = row.get(1)?;
            Ok((key, val))
        })?;
        let mut map = serde_json::Map::new();
        for row in rows {
            let (key, val) = row.or_else(|e| Err(rusqlite::Error::InvalidParameterName(e.to_string())))?;
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&val) {
                map.insert(key, v);
            }
        }
        Ok(serde_json::Value::Object(map))
    }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: serde_json::Value) -> Result<(), String> {
    if let Some(obj) = settings.as_object() {
        state.db.with_conn(|conn| {
            for (key, value) in obj {
                let json_str = serde_json::to_string(value).unwrap_or_default();
                conn.execute(
                    "INSERT OR REPLACE INTO settings (key, value_json, updated_at) VALUES (?1, ?2, datetime('now'))",
                    rusqlite::params![key, json_str],
                )?;
            }
            Ok::<_, rusqlite::Error>(())
        }).map_err(|e| e.to_string())?;
    }
    Ok(())
}
