use crate::state::watcher::FileWatcherState;

/// Start watching a file for external changes (called when a tab opens it).
#[tauri::command]
pub fn watch_file(state: tauri::State<'_, FileWatcherState>, path: String) -> Result<(), String> {
  let canonical = crate::commands::file::validate_path(&path)?;
  state.add(&canonical)
}

/// Stop watching a file (called when its tab closes). The file may already
/// be deleted externally — best-effort canonicalization, never an error.
#[tauri::command]
pub fn unwatch_file(state: tauri::State<'_, FileWatcherState>, path: String) {
  let p = std::path::PathBuf::from(&path);
  let canonical = std::fs::canonicalize(&p).unwrap_or(p);
  state.remove(&canonical);
}
