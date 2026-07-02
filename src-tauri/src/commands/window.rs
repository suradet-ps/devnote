#[tauri::command]
pub fn set_window_title(window: tauri::Window, title: String) -> Result<(), String> {
  window.set_title(&title).map_err(|e| e.to_string())
}
