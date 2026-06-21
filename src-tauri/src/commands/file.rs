use encoding_rs::Encoding;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri_plugin_dialog::DialogExt;
use tempfile::NamedTempFile;

/// Hard cap on file size. Above this, refuse to open.
pub const HARD_LIMIT_BYTES: u64 = 200 * 1024 * 1024; // 200 MB
/// Soft cap: frontend prompts the user to confirm before opening.
pub const SOFT_LIMIT_BYTES: u64 = 10 * 1024 * 1024; // 10 MB
/// Number of bytes to inspect for binary detection.
const BINARY_PROBE_BYTES: usize = 8192;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePayload {
    pub path: String,
    pub content: String,
    pub file_name: String,
    pub encoding: String,
    pub line_ending: String,
}

/// Validate that the path is absolute, exists, and is a regular file.
/// Canonicalizes the path to prevent symlink-based escapes.
pub fn validate_path(path_str: &str) -> Result<PathBuf, String> {
    if path_str.is_empty() {
        return Err("Path is empty".to_string());
    }
    let pb = PathBuf::from(path_str);
    if !pb.is_absolute() {
        return Err("Path must be absolute".to_string());
    }
    let canonical = std::fs::canonicalize(&pb).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "File not found".to_string()
        } else {
            format!("Invalid file path: {}", e)
        }
    })?;
    let meta = std::fs::metadata(&canonical).map_err(|e| format!("Failed to stat path: {}", e))?;
    if !meta.is_file() {
        return Err("Path is not a regular file".to_string());
    }
    Ok(canonical)
}

fn detect_encoding(bytes: &[u8]) -> &'static Encoding {
    if bytes.is_empty() {
        return encoding_rs::UTF_8;
    }
    // Fast path: BOM or valid UTF-8
    if bytes.starts_with(b"\xEF\xBB\xBF") {
        return encoding_rs::UTF_8;
    }
    if std::str::from_utf8(bytes).is_ok() {
        return encoding_rs::UTF_8;
    }
    let (enc, _confidence, _has_bom) = chardet::detect(bytes);
    match enc.as_str() {
        "UTF-8" | "ascii" | "ASCII" => encoding_rs::UTF_8,
        "UTF-16LE" => encoding_rs::UTF_16LE,
        "UTF-16BE" => encoding_rs::UTF_16BE,
        "windows-1252" | "ISO-8859-1" | "iso-8859-1" | "ISO-8859-15" => encoding_rs::WINDOWS_1252,
        _ => encoding_rs::UTF_8,
    }
}

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

fn is_binary(bytes: &[u8]) -> bool {
    let probe_end = bytes.len().min(BINARY_PROBE_BYTES);
    bytes[..probe_end].contains(&0)
}

fn ensure_extension(path_str: &str, default_ext: &str) -> String {
    let path = PathBuf::from(path_str);
    let parent = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    if file_name.is_empty() {
        return path_str.to_string();
    }

    // Hidden files (starting with a dot, e.g. ".env") are kept as-is
    if file_name.starts_with('.') {
        return path_str.to_string();
    }

    let visible = file_name.trim_start_matches('.');
    if !visible.contains('.') {
        let new_name = format!("{}.{}", file_name, default_ext);
        return parent.join(new_name).to_string_lossy().replace('\\', "/");
    }

    path_str.to_string()
}

fn encode_content(content: &str, line_ending: &str, encoding: &str) -> Vec<u8> {
    let normalized: String = match line_ending {
        "CRLF" => normalize_line_endings(content, "\r\n"),
        "CR" => normalize_line_endings(content, "\r"),
        _ => content.to_string(),
    };
    let enc: &'static Encoding = match encoding {
        "windows-1252" | "Windows-1252" => encoding_rs::WINDOWS_1252,
        "UTF-16LE" => encoding_rs::UTF_16LE,
        "UTF-16BE" => encoding_rs::UTF_16BE,
        _ => encoding_rs::UTF_8,
    };
    let (encoded, _enc, _had_error) = enc.encode(&normalized);
    encoded.to_vec()
}

fn normalize_line_endings(s: &str, target: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, chars.peek().copied()) {
            ('\r', Some('\n')) => {
                out.push_str(target);
                chars.next();
            }
            ('\r', _) | ('\n', _) => {
                out.push_str(target);
            }
            (c, _) => out.push(c),
        }
    }
    out
}

#[tauri::command]
pub async fn open_file(app: tauri::AppHandle) -> Result<Option<FilePayload>, String> {
    log::info!("[open_file] Starting file dialog...");
    let file_path = tokio::task::spawn_blocking(move || {
        log::info!("[open_file] Showing native file dialog...");
        let result = app.dialog().file().blocking_pick_file();
        log::info!("[open_file] Dialog returned: {:?}", result.is_some());
        result
    })
    .await
    .map_err(|e| {
        log::error!("[open_file] Dialog task failed: {}", e);
        format!("Dialog task failed: {}", e)
    })?;

    match file_path {
        Some(path) => {
            let path_buf = path
                .into_path()
                .map_err(|e| format!("Invalid file path: {:?}", e))?;
            let path_str = path_buf.to_string_lossy().to_string();
            log::info!("[open_file] Reading file: {}", path_str);
            let result = read_file_internal(&path_str).await;
            match &result {
                Ok(_) => log::info!("[open_file] File read successfully"),
                Err(e) => log::error!("[open_file] Failed to read: {}", e),
            }
            result.map(Some)
        }
        None => {
            log::info!("[open_file] User cancelled dialog");
            Ok(None)
        }
    }
}

#[tauri::command]
pub async fn read_file(path: String) -> Result<FilePayload, String> {
    read_file_internal(&path).await
}

async fn read_file_internal(path: &str) -> Result<FilePayload, String> {
    let canonical = validate_path(path)?;

    let metadata = tokio::fs::metadata(&canonical)
        .await
        .map_err(|e| format!("Failed to stat file: {}", e))?;
    let size = metadata.len();
    if size > HARD_LIMIT_BYTES {
        return Err(format!(
            "File too large: {:.1} MB (limit: {:.0} MB)",
            size as f64 / 1_048_576.0,
            HARD_LIMIT_BYTES as f64 / 1_048_576.0,
        ));
    }
    if size > SOFT_LIMIT_BYTES {
        log::warn!(
            "[read_file] Opening large file: {:.1} MB",
            size as f64 / 1_048_576.0
        );
    }

    let bytes = tokio::fs::read(&canonical)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;

    if is_binary(&bytes) {
        return Err("Refusing to open: file appears to be binary".to_string());
    }

    let encoding = detect_encoding(&bytes);
    let encoding_name = encoding.name().to_string();
    let (content, _enc, _had_errors) = encoding.decode(&bytes);
    let line_ending = detect_line_ending(&content).to_string();
    let file_name = canonical
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "untitled".to_string());

    Ok(FilePayload {
        path: canonical.to_string_lossy().to_string(),
        content: content.to_string(),
        file_name,
        encoding: encoding_name,
        line_ending,
    })
}

#[tauri::command]
pub async fn save_file(
    path: String,
    content: String,
    line_ending: Option<String>,
    encoding: Option<String>,
) -> Result<(), String> {
    let le = line_ending.unwrap_or_else(|| "LF".to_string());
    let enc = encoding.unwrap_or_else(|| "UTF-8".to_string());

    let p = PathBuf::from(&path);
    if !p.is_absolute() {
        return Err("Path must be absolute".to_string());
    }
    if let Some(parent) = p.parent()
        && !parent.as_os_str().is_empty()
        && !Path::new(parent).exists()
    {
        return Err("Parent directory does not exist".to_string());
    }

    let data = encode_content(&content, &le, &enc);
    write_atomic(&p, &data).await
}

#[tauri::command]
pub async fn save_file_as(
    app: tauri::AppHandle,
    content: String,
    suggested_name: Option<String>,
    line_ending: Option<String>,
    encoding: Option<String>,
) -> Result<Option<String>, String> {
    let file_path = tokio::task::spawn_blocking(move || {
        let mut dialog = app.dialog().file();
        if let Some(name) = suggested_name {
            dialog = dialog.set_file_name(&name);
        }
        dialog = dialog
            .add_filter(
                "Text Files",
                &[
                    "txt", "md", "log", "csv", "tsv", "ini", "cfg", "conf", "env", "rst",
                ],
            )
            .add_filter(
                "Source Code",
                &[
                    "rs", "ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "pyw", "go", "rb", "java",
                    "c", "cpp", "cc", "h", "hpp", "php", "sh", "bash", "zsh", "fish",
                ],
            )
            .add_filter("Web Files", &["html", "htm", "css", "scss", "less", "svg"])
            .add_filter(
                "Data Files",
                &[
                    "json", "jsonc", "xml", "toml", "yaml", "yml", "sql", "graphql", "gql",
                ],
            );
        dialog.blocking_save_file()
    })
    .await
    .map_err(|e| format!("Dialog task failed: {}", e))?;

    match file_path {
        Some(path) => {
            let path_buf = path
                .into_path()
                .map_err(|e| format!("Invalid file path: {:?}", e))?;
            let path_str = ensure_extension(&path_buf.to_string_lossy(), "txt");

            let le = line_ending.unwrap_or_else(|| "LF".to_string());
            let enc = encoding.unwrap_or_else(|| "UTF-8".to_string());
            let data = encode_content(&content, &le, &enc);
            let p = PathBuf::from(&path_str);
            write_atomic(&p, &data).await?;
            Ok(Some(path_str))
        }
        None => Ok(None),
    }
}

/// Atomic write: write to a uniquely-named temp file in the same directory,
/// fsync, then rename over the target. Falls back to copy+delete on
/// cross-filesystem rename (EXDEV).
async fn write_atomic(target: &Path, data: &[u8]) -> Result<(), String> {
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let target_buf = target.to_path_buf();
    let parent_for_task = parent.clone();
    let data_vec = data.to_vec();

    let persist_outcome =
        tokio::task::spawn_blocking(move || -> Result<(), (PathBuf, std::io::Error)> {
            let mut temp = NamedTempFile::new_in(&parent_for_task)
                .map_err(|e| (parent_for_task.clone(), e))?;
            temp.as_file_mut()
                .write_all(&data_vec)
                .map_err(|e| (temp.path().to_path_buf(), e))?;
            temp.as_file_mut()
                .sync_all()
                .map_err(|e| (temp.path().to_path_buf(), e))?;
            match temp.persist(&target_buf) {
                Ok(_) => Ok(()),
                Err(persist_err) => {
                    let src_path = persist_err.file.path().to_path_buf();
                    Err((src_path, persist_err.error))
                }
            }
        })
        .await
        .map_err(|e| format!("Failed to save file task: {}", e))?;

    match persist_outcome {
        Ok(()) => Ok(()),
        Err((_, e)) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            Err("Permission denied: the file is read-only. Save a copy instead?".to_string())
        }
        Err((src, e)) => {
            // EXDEV or other cross-FS rename failure: fall back to copy+delete.
            log::warn!(
                "[write_atomic] persist failed ({}), falling back to copy+delete",
                e
            );
            let target_for_copy = target.to_path_buf();
            tokio::fs::copy(&src, &target_for_copy)
                .await
                .map_err(|e2| format!("Failed to save (rename: {}, copy: {})", e, e2))?;
            let _ = tokio::fs::remove_file(&src).await;
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn check_file_size(path: String) -> Result<String, String> {
    // Returns decimal string to avoid JS Number precision loss on files > 2^53
    let canonical = validate_path(&path)?;
    let len = tokio::fs::metadata(&canonical)
        .await
        .map_err(|e| format!("Failed to check file: {}", e))?
        .len();
    Ok(len.to_string())
}

#[tauri::command]
pub async fn add_recent_file(
    state: tauri::State<'_, crate::state::recent::RecentFilesState>,
    path: String,
) -> Result<(), String> {
    // Validate before adding to prevent poisoned recent list
    validate_path(&path)?;
    {
        let mut files = state.files.lock().map_err(|e| e.to_string())?;
        files.retain(|p| p != &path);
        files.insert(0, path);

        if files.len() > crate::state::recent::RecentFilesState::MAX_ENTRIES {
            files.truncate(crate::state::recent::RecentFilesState::MAX_ENTRIES);
        }
    }
    state.persist()?;
    Ok(())
}

#[tauri::command]
pub async fn get_recent_files(
    state: tauri::State<'_, crate::state::recent::RecentFilesState>,
) -> Result<Vec<String>, String> {
    let files = state.files.lock().map_err(|e| e.to_string())?;
    Ok(files.clone())
}

#[tauri::command]
pub async fn remove_recent_file(
    state: tauri::State<'_, crate::state::recent::RecentFilesState>,
    path: String,
) -> Result<(), String> {
    {
        let mut files = state.files.lock().map_err(|e| e.to_string())?;
        files.retain(|p| p != &path);
    }
    state.persist()?;
    Ok(())
}

#[tauri::command]
pub async fn get_pending_files(
    state: tauri::State<'_, crate::PendingFilesState>,
) -> Result<Vec<String>, String> {
    let pending = state.drain();
    log::info!("[get_pending_files] returning {} files", pending.len());
    Ok(pending)
}

/// Frontend signals it is ready to receive any pending file-open events.
/// Rust drains the pending list, then re-emits each file via the
/// `file-opened` Tauri event so the frontend's `listen('file-opened')`
/// handler can pick them up. This closes the race where the
/// frontend's listener might not yet be registered when the
/// underlying Apple Event / RunEvent::Opened is processed.
#[tauri::command]
pub async fn frontend_ready(
    state: tauri::State<'_, crate::PendingFilesState>,
    app: tauri::AppHandle,
) -> Result<Vec<String>, String> {
    use tauri::Manager;
    let pending = state.drain();
    log::info!(
        "[frontend_ready] frontend signaled ready, delivering {} pending files",
        pending.len()
    );
    if !pending.is_empty()
        && let Some(window) = app.get_webview_window("main")
    {
        use tauri::Emitter;
        window.emit("file-opened", &pending).ok();
    }
    Ok(pending)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_line_ending_prefers_crlf() {
        assert_eq!(detect_line_ending("a\r\nb\r\nc"), "CRLF");
        assert_eq!(detect_line_ending("a\nb\nc"), "LF");
        assert_eq!(detect_line_ending("a\rb\rc"), "CR");
        assert_eq!(detect_line_ending("a\r\nb\nc\r\nd"), "CRLF");
        assert_eq!(detect_line_ending(""), "LF");
    }

    #[test]
    fn ensure_extension_appends_only_when_missing() {
        assert_eq!(ensure_extension("/a/b", "txt"), "/a/b.txt");
        assert_eq!(ensure_extension("/a/.env", "txt"), "/a/.env");
        assert_eq!(ensure_extension("/a/notes.md", "txt"), "/a/notes.md");
        assert_eq!(ensure_extension("/a/.gitignore", "txt"), "/a/.gitignore");
        assert_eq!(
            ensure_extension("/a/archive.tar.gz", "txt"),
            "/a/archive.tar.gz"
        );
    }

    #[test]
    fn normalize_line_endings_crlf() {
        assert_eq!(normalize_line_endings("a\nb\nc", "\r\n"), "a\r\nb\r\nc");
        assert_eq!(normalize_line_endings("a\r\nb\nc", "\r\n"), "a\r\nb\r\nc");
        assert_eq!(normalize_line_endings("a\rb\nc", "\r\n"), "a\r\nb\r\nc");
    }

    #[test]
    fn validate_path_rejects_empty_and_relative() {
        assert!(validate_path("").is_err());
        assert!(validate_path("relative/path").is_err());
    }

    #[test]
    fn validate_path_accepts_existing_file() {
        let dir = std::env::temp_dir();
        let f = dir.join("devnote-test-validate.txt");
        std::fs::write(&f, b"hello").unwrap();
        let result = validate_path(&f.to_string_lossy());
        assert!(result.is_ok());
        let _ = std::fs::remove_file(&f);
    }

    #[test]
    fn is_binary_detects_nul_byte() {
        assert!(is_binary(b"hello\x00world"));
        assert!(!is_binary(b"hello world\n"));
        assert!(!is_binary(b""));
    }
}
