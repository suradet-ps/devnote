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
  // UTF-16 BOMs take precedence over the chardet fallback
  if bytes.starts_with(b"\xFF\xFE") {
    return encoding_rs::UTF_16LE;
  }
  if bytes.starts_with(b"\xFE\xFF") {
    return encoding_rs::UTF_16BE;
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

/// UTF-16 has no encoder in `encoding_rs` (encoders always output UTF-8), so
/// UTF-16 byte sequences must be produced manually from the string's code units.
fn utf16_bytes(s: &str, big_endian: bool) -> Vec<u8> {
  let mut out = Vec::with_capacity(s.len() * 2);
  for unit in s.encode_utf16() {
    if big_endian {
      out.extend_from_slice(&unit.to_be_bytes());
    } else {
      out.extend_from_slice(&unit.to_le_bytes());
    }
  }
  out
}

fn encode_content(content: &str, line_ending: &str, encoding: &str) -> Vec<u8> {
  // "LF" must normalize too — otherwise a CRLF-loaded buffer stays CRLF
  // even when the user explicitly saves with LF endings.
  let normalized: String = match line_ending {
    "CRLF" => normalize_line_endings(content, "\r\n"),
    "CR" => normalize_line_endings(content, "\r"),
    _ => normalize_line_endings(content, "\n"),
  };
  match encoding {
    "UTF-16LE" => {
      let mut out = vec![0xFF, 0xFE];
      out.extend_from_slice(&utf16_bytes(&normalized, false));
      out
    },
    "UTF-16BE" => {
      let mut out = vec![0xFE, 0xFF];
      out.extend_from_slice(&utf16_bytes(&normalized, true));
      out
    },
    "windows-1252" | "Windows-1252" => encoding_rs::WINDOWS_1252.encode(&normalized).0.to_vec(),
    _ => normalized.as_bytes().to_vec(),
  }
}

fn normalize_line_endings(s: &str, target: &str) -> String {
  let mut out = String::with_capacity(s.len());
  let mut chars = s.chars().peekable();
  while let Some(c) = chars.next() {
    match (c, chars.peek().copied()) {
      ('\r', Some('\n')) => {
        out.push_str(target);
        chars.next();
      },
      ('\r', _) | ('\n', _) => {
        out.push_str(target);
      },
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
    },
    None => {
      log::info!("[open_file] User cancelled dialog");
      Ok(None)
    },
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

  // UTF-16 text is full of NUL bytes — that is structural, not binary.
  let has_utf16_bom = bytes.starts_with(b"\xFF\xFE") || bytes.starts_with(b"\xFE\xFF");
  if is_binary(&bytes) && !has_utf16_bom {
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
          "rs", "ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "pyw", "go", "rb", "java", "c",
          "cpp", "cc", "h", "hpp", "php", "sh", "bash", "zsh", "fish",
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
    },
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
      let mut temp =
        NamedTempFile::new_in(&parent_for_task).map_err(|e| (parent_for_task.clone(), e))?;
      temp
        .as_file_mut()
        .write_all(&data_vec)
        .map_err(|e| (temp.path().to_path_buf(), e))?;
      temp
        .as_file_mut()
        .sync_all()
        .map_err(|e| (temp.path().to_path_buf(), e))?;
      match temp.persist(&target_buf) {
        Ok(_) => Ok(()),
        Err(persist_err) => {
          let src_path = persist_err.file.path().to_path_buf();
          Err((src_path, persist_err.error))
        },
      }
    })
    .await
    .map_err(|e| format!("Failed to save file task: {}", e))?;

  match persist_outcome {
    Ok(()) => Ok(()),
    Err((_, e)) if e.kind() == std::io::ErrorKind::PermissionDenied => {
      Err("Permission denied: the file is read-only. Save a copy instead?".to_string())
    },
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
    },
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
  use tempfile::TempDir;

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

  #[test]
  fn detect_encoding_prefers_utf16_boms_over_chardet() {
    assert_eq!(detect_encoding(b"\xFF\xFEh\x00i\x00"), encoding_rs::UTF_16LE);
    assert_eq!(detect_encoding(b"\xFE\xFF\x00h\x00i"), encoding_rs::UTF_16BE);
    assert_eq!(detect_encoding(b"\xEF\xBB\xBFhi"), encoding_rs::UTF_8);
    assert_eq!(detect_encoding(b""), encoding_rs::UTF_8);
  }

  #[test]
  fn encode_content_emits_utf16_boms() {
    assert_eq!(encode_content("hi", "LF", "UTF-16LE"), b"\xFF\xFEh\x00i\x00");
    assert_eq!(encode_content("hi", "LF", "UTF-16BE"), b"\xFE\xFF\x00h\x00i");
    assert_eq!(encode_content("hi", "LF", "UTF-8"), b"hi");
    assert_eq!(encode_content("hi", "LF", "windows-1252"), b"hi");
  }

  fn win1252_sample() -> String {
    "L'été à Montréal était très chaleureux.\nLes éléphants et les éléphantes.\n".to_string()
  }

  async fn write_file(dir: &TempDir, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let path = dir.path().join(name);
    tokio::fs::write(&path, bytes).await.unwrap();
    path
  }

  #[tokio::test]
  async fn read_encoding_matrix_utf8_utf16_and_windows1252() {
    let dir = TempDir::new().unwrap();
    let sample = win1252_sample();

    // UTF-8 (no BOM)
    let p = write_file(&dir, "a.txt", sample.as_bytes()).await;
    let payload = read_file_internal(&p.to_string_lossy()).await.unwrap();
    assert_eq!(payload.encoding, "UTF-8");
    assert_eq!(payload.content, sample);

    // UTF-8 with BOM — BOM stripped, encoding reported as UTF-8
    let mut bom = b"\xEF\xBB\xBF".to_vec();
    bom.extend_from_slice(sample.as_bytes());
    let p = write_file(&dir, "b.txt", &bom).await;
    let payload = read_file_internal(&p.to_string_lossy()).await.unwrap();
    assert_eq!(payload.encoding, "UTF-8");
    assert_eq!(payload.content, sample);

    // UTF-16LE with BOM (NUL bytes must NOT be treated as binary)
    let mut le_bom = b"\xFF\xFE".to_vec();
    le_bom.extend_from_slice(&utf16_bytes(&sample, false));
    let p = write_file(&dir, "c.txt", &le_bom).await;
    let payload = read_file_internal(&p.to_string_lossy()).await.unwrap();
    assert_eq!(payload.encoding, "UTF-16LE");
    assert_eq!(payload.content, sample);

    // UTF-16BE with BOM
    let mut be_bom = b"\xFE\xFF".to_vec();
    be_bom.extend_from_slice(&utf16_bytes(&sample, true));
    let p = write_file(&dir, "d.txt", &be_bom).await;
    let payload = read_file_internal(&p.to_string_lossy()).await.unwrap();
    assert_eq!(payload.encoding, "UTF-16BE");
    assert_eq!(payload.content, sample);

    // Windows-1252 (invalid UTF-8, chardet fallback)
    let (w, _, _) = encoding_rs::WINDOWS_1252.encode(&sample);
    let p = write_file(&dir, "e.txt", &w).await;
    let payload = read_file_internal(&p.to_string_lossy()).await.unwrap();
    assert_eq!(payload.encoding, "windows-1252");
    assert_eq!(payload.content, sample);
  }

  #[tokio::test]
  async fn save_read_round_trip_preserves_encoding_and_line_endings() {
    let dir = TempDir::new().unwrap();
    let sample = win1252_sample();
    for (encoding, expected_name) in [
      ("UTF-8", "UTF-8"),
      ("UTF-16LE", "UTF-16LE"),
      ("UTF-16BE", "UTF-16BE"),
      ("windows-1252", "windows-1252"),
    ] {
      let path = dir.path().join(format!("rt-{}.txt", encoding));
      save_file(
        path.to_string_lossy().to_string(),
        sample.clone(),
        Some("LF".to_string()),
        Some(encoding.to_string()),
      )
      .await
      .unwrap();
      let payload = read_file_internal(&path.to_string_lossy()).await.unwrap();
      assert_eq!(payload.encoding, expected_name, "encoding {}", encoding);
      assert_eq!(payload.content, sample, "content for {}", encoding);
      assert_eq!(payload.line_ending, "LF");
    }
  }

  #[tokio::test]
  async fn line_ending_detection_and_round_trip() {
    let dir = TempDir::new().unwrap();

    let p = write_file(&dir, "lf.txt", b"a\nb\nc\n").await;
    let payload = read_file_internal(&p.to_string_lossy()).await.unwrap();
    assert_eq!(payload.line_ending, "LF");

    let p = write_file(&dir, "crlf.txt", b"a\r\nb\r\nc\r\n").await;
    let payload = read_file_internal(&p.to_string_lossy()).await.unwrap();
    assert_eq!(payload.line_ending, "CRLF");

    let p = write_file(&dir, "cr.txt", b"a\rb\rc\r").await;
    let payload = read_file_internal(&p.to_string_lossy()).await.unwrap();
    assert_eq!(payload.line_ending, "CR");

    // Re-saving with a different target normalizes the bytes
    let path = dir.path().join("crlf2.txt");
    save_file(
      path.to_string_lossy().to_string(),
      "a\r\nb\r\nc\r\n".to_string(),
      Some("LF".to_string()),
      Some("UTF-8".to_string()),
    )
    .await
    .unwrap();
    let bytes = tokio::fs::read(&path).await.unwrap();
    assert_eq!(bytes, b"a\nb\nc\n");
  }

  #[tokio::test]
  async fn binary_nul_file_is_rejected_without_utf16_bom() {
    let dir = TempDir::new().unwrap();
    let p = write_file(&dir, "bin.dat", b"\x00\x01\x02\x03").await;
    let err = read_file_internal(&p.to_string_lossy()).await.unwrap_err();
    assert!(err.contains("binary"), "unexpected error: {}", err);
  }

  #[tokio::test]
  async fn hard_cap_rejects_files_over_200mb() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("huge.txt");
    let f = std::fs::File::create(&path).unwrap();
    f.set_len(HARD_LIMIT_BYTES + 1).unwrap();
    drop(f);
    let err = read_file_internal(&path.to_string_lossy()).await.unwrap_err();
    assert!(err.contains("File too large"), "unexpected error: {}", err);
  }

  #[tokio::test]
  async fn soft_cap_allows_files_over_10mb() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("big.csv");
    let content = "a,b\r\n".repeat((SOFT_LIMIT_BYTES / 4) as usize + 1);
    tokio::fs::write(&path, content.as_bytes()).await.unwrap();
    assert!(content.len() as u64 > SOFT_LIMIT_BYTES);
    let payload = read_file_internal(&path.to_string_lossy()).await.unwrap();
    assert_eq!(payload.content.len(), content.len());
    assert_eq!(payload.line_ending, "CRLF");
    assert_eq!(payload.encoding, "UTF-8");
  }

  #[tokio::test]
  async fn atomic_save_leaves_no_temp_files_behind() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("t.txt");
    save_file(
      path.to_string_lossy().to_string(),
      "hello".to_string(),
      Some("LF".to_string()),
      Some("UTF-8".to_string()),
    )
    .await
    .unwrap();
    let names: Vec<String> = std::fs::read_dir(dir.path())
      .unwrap()
      .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
      .collect();
    assert_eq!(names, vec!["t.txt"]);
    assert_eq!(tokio::fs::read_to_string(&path).await.unwrap(), "hello");
  }

  #[cfg(windows)]
  #[tokio::test]
  async fn failed_save_leaves_target_untouched() {
    use std::fs::Permissions;

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("locked.txt");
    std::fs::write(&path, b"original").unwrap();
    // Locking the file makes the atomic rename fail on Windows
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&path, perms).unwrap();

    let err = save_file(
      path.to_string_lossy().to_string(),
      "overwritten".to_string(),
      Some("LF".to_string()),
      Some("UTF-8".to_string()),
    )
    .await
    .unwrap_err();
    assert!(
      err.contains("Permission denied"),
      "unexpected error: {}",
      err
    );

    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_readonly(false);
    std::fs::set_permissions(&path, perms).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "original");
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn validate_path_canonicalizes_symlinks() {
    use std::os::unix::fs::symlink;

    let dir = TempDir::new().unwrap();
    let real = dir.path().join("real.txt");
    std::fs::write(&real, b"x").unwrap();
    let link = dir.path().join("link.txt");
    symlink(&real, &link).unwrap();

    let canonical = validate_path(&link.to_string_lossy()).unwrap();
    assert_eq!(canonical, real.canonicalize().unwrap());
  }
}
