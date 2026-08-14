#[tauri::command]
pub fn set_window_title(window: tauri::Window, title: String) -> Result<(), String> {
  window.set_title(&title).map_err(|e| e.to_string())
}

/// Open the OS print dialog for the current webview (print to PDF, etc.).
/// The JS webview API does not expose `print()` yet, so we wrap the Rust API.
#[tauri::command]
pub fn print_current(webview: tauri::Webview) -> Result<(), String> {
  webview.print().map_err(|e| e.to_string())
}
