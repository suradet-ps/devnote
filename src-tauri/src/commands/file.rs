use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri_plugin_dialog::DialogExt;

#[derive(Debug, Serialize, Deserialize)]
pub struct FilePayload {
    pub path: String,
    pub content: String,
    pub file_name: String,
}

#[tauri::command]
pub async fn open_file(app: tauri::AppHandle) -> Result<Option<FilePayload>, String> {
    let file_path = app
        .dialog()
        .file()
        .add_filter("All Files", &["*"])
        .blocking_pick_file();

    match file_path {
        Some(path) => {
            let path_str = path.to_string().to_string();
            let content = tokio::fs::read_to_string(&path_str)
                .await
                .map_err(|e| e.to_string())?;
            let file_name = PathBuf::from(&path_str)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "untitled".to_string());

            Ok(Some(FilePayload {
                path: path_str,
                content,
                file_name,
            }))
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn read_file(path: String) -> Result<FilePayload, String> {
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;
    let file_name = PathBuf::from(&path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "untitled".to_string());

    Ok(FilePayload {
        path,
        content,
        file_name,
    })
}

#[tauri::command]
pub async fn save_file(path: String, content: String) -> Result<(), String> {
    tokio::fs::write(&path, &content)
        .await
        .map_err(|e| format!("Failed to save file: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn save_file_as(
    app: tauri::AppHandle,
    content: String,
    suggested_name: Option<String>,
) -> Result<Option<String>, String> {
    let mut dialog = app.dialog().file();

    if let Some(name) = suggested_name {
        dialog = dialog.set_file_name(&name);
    }

    let file_path = dialog.blocking_save_file();

    match file_path {
        Some(path) => {
            let path_str = path.to_string().to_string();
            tokio::fs::write(&path_str, &content)
                .await
                .map_err(|e| format!("Failed to save file: {}", e))?;
            Ok(Some(path_str))
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn add_recent_file(
    state: tauri::State<'_, crate::state::recent::RecentFilesState>,
    path: String,
) -> Result<(), String> {
    let mut files = state.0.lock().map_err(|e| e.to_string())?;

    files.retain(|p| p != &path);
    files.insert(0, path);

    if files.len() > 10 {
        files.truncate(10);
    }

    Ok(())
}

#[tauri::command]
pub async fn get_recent_files(
    state: tauri::State<'_, crate::state::recent::RecentFilesState>,
) -> Result<Vec<String>, String> {
    let files = state.0.lock().map_err(|e| e.to_string())?;
    Ok(files.clone())
}

#[tauri::command]
pub async fn remove_recent_file(
    state: tauri::State<'_, crate::state::recent::RecentFilesState>,
    path: String,
) -> Result<(), String> {
    let mut files = state.0.lock().map_err(|e| e.to_string())?;
    files.retain(|p| p != &path);
    Ok(())
}
