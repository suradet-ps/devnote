use crate::state::recovery::{RecoveryEntry, RecoveryState};
use std::path::Path;
use tauri::Manager;

/// Persist recovery entries as `recovery.json` under `dir`.
/// Extracted from the command so it can be tested without a Tauri runtime.
pub async fn write_recovery_file(dir: &Path, entries: &[RecoveryEntry]) -> Result<(), String> {
  tokio::fs::create_dir_all(dir)
    .await
    .map_err(|e| format!("Failed to create recovery dir: {}", e))?;

  let path = dir.join("recovery.json");
  let json = serde_json::to_string(entries).map_err(|e| e.to_string())?;
  tokio::fs::write(&path, json)
    .await
    .map_err(|e| format!("Failed to write recovery data: {}", e))?;
  Ok(())
}

/// Read recovery entries from `dir`. Missing file, empty list, and malformed
/// JSON all resolve to `Ok(None)` — a corrupt recovery file must never block
/// startup.
pub async fn read_recovery_file(dir: &Path) -> Result<Option<Vec<RecoveryEntry>>, String> {
  let path = dir.join("recovery.json");
  if !path.exists() {
    return Ok(None);
  }

  let json = tokio::fs::read_to_string(&path)
    .await
    .map_err(|e| format!("Failed to read recovery data: {}", e))?;

  match serde_json::from_str::<Vec<RecoveryEntry>>(&json) {
    Ok(entries) if !entries.is_empty() => Ok(Some(entries)),
    Ok(_) => Ok(None),
    Err(err) => {
      log::warn!("[read_recovery_file] parse error: {}", err);
      Ok(None)
    },
  }
}

/// Remove the recovery file. Missing file is not an error.
pub async fn clear_recovery_file(dir: &Path) -> Result<(), String> {
  let path = dir.join("recovery.json");
  if path.exists() {
    tokio::fs::remove_file(&path)
      .await
      .map_err(|e| format!("Failed to clear recovery data: {}", e))?;
  }
  Ok(())
}

#[tauri::command]
pub async fn save_recovery_data(
  app: tauri::AppHandle,
  tabs: Vec<RecoveryEntry>,
) -> Result<(), String> {
  let state = app.state::<RecoveryState>();
  let dir = state.dir.clone();

  // Update in-memory state (drop lock before await)
  {
    let mut entries = state.entries.lock().map_err(|e| e.to_string())?;
    *entries = tabs.clone();
  }

  write_recovery_file(&dir, &tabs).await
}

#[tauri::command]
pub async fn check_recovery_data(
  app: tauri::AppHandle,
) -> Result<Option<Vec<RecoveryEntry>>, String> {
  let state = app.state::<RecoveryState>();
  read_recovery_file(&state.dir).await
}

#[tauri::command]
pub async fn clear_recovery_data(app: tauri::AppHandle) -> Result<(), String> {
  let state = app.state::<RecoveryState>();
  let dir = state.dir.clone();

  clear_recovery_file(&dir).await?;

  let mut entries = state.entries.lock().map_err(|e| e.to_string())?;
  entries.clear();
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::TempDir;

  fn entry(file_name: &str, content: &str, path: Option<&str>, saved_at: &str) -> RecoveryEntry {
    RecoveryEntry {
      file_name: file_name.to_string(),
      content: content.to_string(),
      path: path.map(|p| p.to_string()),
      saved_at: saved_at.to_string(),
    }
  }

  #[tokio::test]
  async fn round_trip_preserves_identical_entries() {
    let dir = TempDir::new().unwrap();
    let entries = vec![
      entry(
        "a.txt",
        "hello world",
        Some("/x/a.txt"),
        "2026-01-01T00:00:00Z",
      ),
      entry("untitled-1", "draft", None, "2026-01-01T00:00:01Z"),
    ];

    write_recovery_file(dir.path(), &entries).await.unwrap();
    let got = read_recovery_file(dir.path())
      .await
      .unwrap()
      .expect("entries");

    assert_eq!(got.len(), 2);
    assert_eq!(got[0].file_name, "a.txt");
    assert_eq!(got[0].content, "hello world");
    assert_eq!(got[0].path.as_deref(), Some("/x/a.txt"));
    assert_eq!(got[0].saved_at, "2026-01-01T00:00:00Z");
    assert_eq!(got[1].path, None);
    assert_eq!(got[1].content, "draft");
  }

  #[tokio::test]
  async fn missing_recovery_file_returns_none() {
    let dir = TempDir::new().unwrap();
    assert_eq!(read_recovery_file(dir.path()).await.unwrap(), None);
  }

  #[tokio::test]
  async fn empty_entries_are_treated_as_no_recovery() {
    let dir = TempDir::new().unwrap();
    write_recovery_file(dir.path(), &[]).await.unwrap();
    assert_eq!(read_recovery_file(dir.path()).await.unwrap(), None);
  }

  #[tokio::test]
  async fn malformed_json_returns_none_instead_of_error() {
    let dir = TempDir::new().unwrap();
    tokio::fs::write(dir.path().join("recovery.json"), b"{ not json")
      .await
      .unwrap();
    assert_eq!(read_recovery_file(dir.path()).await.unwrap(), None);
  }

  #[tokio::test]
  async fn clear_removes_file_and_entries() {
    let dir = TempDir::new().unwrap();
    write_recovery_file(dir.path(), &[entry("a", "x", None, "t")])
      .await
      .unwrap();
    clear_recovery_file(dir.path()).await.unwrap();
    assert!(!dir.path().join("recovery.json").exists());
    assert_eq!(read_recovery_file(dir.path()).await.unwrap(), None);
  }

  #[tokio::test]
  async fn data_survives_when_not_cleared() {
    // "Cancel" in the restore dialog performs no clear — recovery must persist.
    let dir = TempDir::new().unwrap();
    write_recovery_file(
      dir.path(),
      &[entry("a.txt", "keep me", Some("/a.txt"), "t")],
    )
    .await
    .unwrap();
    let got = read_recovery_file(dir.path()).await.unwrap().unwrap();
    assert_eq!(got[0].content, "keep me");
  }

  #[tokio::test]
  async fn clear_is_idempotent_when_file_missing() {
    let dir = TempDir::new().unwrap();
    clear_recovery_file(dir.path()).await.unwrap();
  }
}
