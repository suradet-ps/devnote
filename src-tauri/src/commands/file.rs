use encoding_rs::Encoding;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri_plugin_dialog::DialogExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePayload {
    pub path: String,
    pub content: String,
    pub file_name: String,
    pub encoding: String,
    pub line_ending: String,
}

/// Detect encoding from raw bytes
fn detect_encoding(bytes: &[u8]) -> &'static Encoding {
    if bytes.is_empty() {
        return encoding_rs::UTF_8;
    }
    let (enc, _confidence, _has_bom) = chardet::detect(bytes);
    match enc.as_str() {
        "UTF-8" => encoding_rs::UTF_8,
        "UTF-16LE" => encoding_rs::UTF_16LE,
        "UTF-16BE" => encoding_rs::UTF_16BE,
        "windows-1252" | "ISO-8859-1" => encoding_rs::WINDOWS_1252,
        _ => encoding_rs::WINDOWS_1252,
    }
}

/// Detect line endings from content string
fn detect_line_ending(content: &str) -> &'static str {
    let crlf_count = content.matches("\r\n").count();
    let lf_count = content.matches('\n').count() - crlf_count;
    let cr_count = content.matches('\r').count() - crlf_count;
    if crlf_count > lf_count && crlf_count > cr_count {
        "CRLF"
    } else if cr_count > lf_count && cr_count > crlf_count {
        "CR"
    } else {
        "LF"
    }
}

/// Encode content into bytes using the specified encoding
fn encode_content(content: &str, line_ending: &str) -> Vec<u8> {
    let normalized = match line_ending {
        "CRLF" => content.replace('\n', "\r\n").replace("\r\r\n", "\r\n"),
        "CR" => content.replace('\n', "\r"),
        _ => content.to_string(),
    };
    let (encoded, _enc, _had_error) = encoding_rs::UTF_8.encode(&normalized);
    encoded.to_vec()
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
            let path_buf = path
                .into_path()
                .map_err(|e| format!("Invalid file path: {:?}", e))?;
            let path_str = path_buf.to_string_lossy().to_string();
            read_file_internal(&path_str).await.map(Some)
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn read_file(path: String) -> Result<FilePayload, String> {
    read_file_internal(&path).await
}

async fn read_file_internal(path: &str) -> Result<FilePayload, String> {
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let encoding = detect_encoding(&bytes);
    let (content, _enc, _had_error) = encoding.decode(&bytes);
    let line_ending = detect_line_ending(&content);
    let file_name = PathBuf::from(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "untitled".to_string());

    Ok(FilePayload {
        path: path.to_string(),
        content: content.to_string(),
        file_name,
        encoding: "UTF-8".to_string(),
        line_ending: line_ending.to_string(),
    })
}

#[tauri::command]
pub async fn read_file_with_encoding(path: String) -> Result<FilePayload, String> {
    read_file_internal(&path).await
}

#[tauri::command]
pub async fn save_file(path: String, content: String) -> Result<(), String> {
    // Atomic save: write to temp file, then rename
    let p = PathBuf::from(&path);
    let parent = p.parent().ok_or("Invalid file path")?;
    let stem = p.file_stem().ok_or("Invalid file path")?;
    let ext = p
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let temp_name = format!(
        ".{}~{:x}{}",
        stem.to_string_lossy(),
        std::process::id(),
        ext
    );
    let temp_path = parent.join(&temp_name);

    let data = encode_content(&content, "LF");
    tokio::fs::write(&temp_path, &data).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            "Permission denied: the file is read-only. Save a copy instead?".to_string()
        } else {
            format!("Failed to save file: {}", e)
        }
    })?;

    tokio::fs::rename(&temp_path, &path)
        .await
        .map_err(|e| format!("Failed to finalize save: {}", e))?;

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
            let path_buf = path
                .into_path()
                .map_err(|e| format!("Invalid file path: {:?}", e))?;
            let path_str = path_buf.to_string_lossy().to_string();

            let data = encode_content(&content, "LF");
            tokio::fs::write(&path_str, &data).await.map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    "Permission denied: the file is read-only. Save a copy instead?".to_string()
                } else {
                    format!("Failed to save file: {}", e)
                }
            })?;
            Ok(Some(path_str))
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn check_file_size(path: String) -> Result<u64, String> {
    let metadata = tokio::fs::metadata(&path)
        .await
        .map_err(|e| format!("Failed to check file: {}", e))?;
    Ok(metadata.len())
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
