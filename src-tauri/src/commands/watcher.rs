use crate::state::watcher::FileWatcherState;

/// Start watching a file for external changes (called when a tab opens it).
#[tauri::command]
pub fn watch_file(state: tauri::State<'_, FileWatcherState>, path: String) -> Result<(), String> {
  let canonical = crate::commands::file::validate_path(&path)?;
  state.add(&canonical)
}

/// Stop watching a file (called when its tab closes).
#[tauri::command]
pub fn unwatch_file(state: tauri::State<'_, FileWatcherState>, path: String) {
  if let Ok(canonical) = crate::commands::file::validate_path(&path) {
    state.remove(&canonical);
  }
}
