use crate::state::recovery::{RecoveryEntry, RecoveryState};
use tauri::Manager;

#[tauri::command]
pub async fn save_recovery_data(
    app: tauri::AppHandle,
    tabs: Vec<RecoveryEntry>,
) -> Result<(), String> {
    let state = app.state::<RecoveryState>();
    let dir = state.dir.clone();

    // Ensure recovery directory exists
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("Failed to create recovery dir: {}", e))?;

    // Update in-memory state (drop lock before await)
    {
        let mut entries = state.entries.lock().map_err(|e| e.to_string())?;
        *entries = tabs.clone();
    }

    let path = dir.join("recovery.json");
    let json = serde_json::to_string(&tabs).map_err(|e| e.to_string())?;
    tokio::fs::write(&path, json)
        .await
        .map_err(|e| format!("Failed to write recovery data: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn check_recovery_data(
    app: tauri::AppHandle,
) -> Result<Option<Vec<RecoveryEntry>>, String> {
    let state = app.state::<RecoveryState>();
    let path = state.dir.join("recovery.json");

    if !path.exists() {
        return Ok(None);
    }

    let json = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("Failed to read recovery data: {}", e))?;

    let entries: Vec<RecoveryEntry> = serde_json::from_str(&json).unwrap_or_default();
    if entries.is_empty() {
        return Ok(None);
    }

    Ok(Some(entries))
}

#[tauri::command]
pub async fn clear_recovery_data(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<RecoveryState>();
    let path = state.dir.join("recovery.json");

    if path.exists() {
        tokio::fs::remove_file(&path)
            .await
            .map_err(|e| format!("Failed to clear recovery data: {}", e))?;
    }

    let mut entries = state.entries.lock().map_err(|e| e.to_string())?;
    entries.clear();

    Ok(())
}

#[tauri::command]
pub async fn get_app_data_dir(app: tauri::AppHandle) -> Result<String, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(dir.to_string_lossy().to_string())
}
