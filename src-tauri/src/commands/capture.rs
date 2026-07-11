/// 截屏命令
#[tauri::command]
pub fn capture_screen() -> Result<Vec<u8>, String> {
    crate::perception::capture_screen().map_err(|e| e.to_string())
}

/// 摄像头命令
#[tauri::command]
pub fn capture_camera() -> Result<Vec<u8>, String> {
    crate::perception::capture_camera().map_err(|e| e.to_string())
}
