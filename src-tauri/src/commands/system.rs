use serde::Serialize;
use tauri::Manager;

#[derive(Serialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub data_dir: String,
}

#[tauri::command]
pub fn health_check() -> String {
    "ok".into()
}

#[tauri::command]
pub fn get_app_info(app: tauri::AppHandle) -> Result<AppInfo, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .into_owned();
    Ok(AppInfo {
        name: "AI 伴侣".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        data_dir,
    })
}
