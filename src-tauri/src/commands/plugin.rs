use crate::plugin::registry::InstalledPlugin;
use crate::state::AppState;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub fn list_plugins(state: State<'_, AppState>) -> Result<Vec<InstalledPlugin>, String> {
    let registry = state
        .plugin_registry
        .read()
        .map_err(|e| format!("锁定插件注册表失败: {e}"))?;
    Ok(registry.list_plugins())
}

#[tauri::command]
pub fn get_plugin(state: State<'_, AppState>, plugin_id: String) -> Result<Option<InstalledPlugin>, String> {
    let registry = state
        .plugin_registry
        .read()
        .map_err(|e| format!("锁定插件注册表失败: {e}"))?;
    Ok(registry.get_plugin(&plugin_id))
}

#[tauri::command]
pub fn install_plugin(
    state: State<'_, AppState>,
    source_path: String,
) -> Result<InstalledPlugin, String> {
    let source = PathBuf::from(source_path);
    let mut registry = state
        .plugin_registry
        .write()
        .map_err(|e| format!("锁定插件注册表失败: {e}"))?;
    registry.install_plugin(&source)
}

#[tauri::command]
pub fn uninstall_plugin(state: State<'_, AppState>, plugin_id: String) -> Result<(), String> {
    let mut registry = state
        .plugin_registry
        .write()
        .map_err(|e| format!("锁定插件注册表失败: {e}"))?;
    registry.uninstall_plugin(&plugin_id)
}

#[tauri::command]
pub fn enable_plugin(
    state: State<'_, AppState>,
    plugin_id: String,
    enabled: bool,
) -> Result<InstalledPlugin, String> {
    let mut registry = state
        .plugin_registry
        .write()
        .map_err(|e| format!("锁定插件注册表失败: {e}"))?;
    registry.set_enabled(&plugin_id, enabled)
}

#[tauri::command]
pub fn configure_plugin(
    state: State<'_, AppState>,
    plugin_id: String,
    config: serde_json::Value,
) -> Result<InstalledPlugin, String> {
    let mut registry = state
        .plugin_registry
        .write()
        .map_err(|e| format!("锁定插件注册表失败: {e}"))?;
    registry.set_config(&plugin_id, config)
}
