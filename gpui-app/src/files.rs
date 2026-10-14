//! File I/O for the pure-Rust port: encoding detection, line-ending
//! preservation and atomic saves, ported from `src-tauri/src/commands/file.rs`.
//!
//! The Tauri version received the content as a command parameter; here we read
//! and write directly, so the same semantics are kept but without IPC.

use std::io::Write;
use std::path::{Path, PathBuf};

use encoding_rs::Encoding;
use serde::{Deserialize, Serialize};

/// Refuse to open files larger than this outright.
pub const HARD_LIMIT_BYTES: u64 = 200 * 1024 * 1024; // 200 MB
/// Warn above this size (parity with the Tauri implementation; the GPUI port
/// does not log yet).
#[allow(dead_code)]
pub const SOFT_LIMIT_BYTES: u64 = 10 * 1024 * 1024; // 10 MB
/// Larger files open as read-only previews.
pub const PREVIEW_LIMIT_BYTES: u64 = 50 * 1024 * 1024; // 50 MB

const BINARY_PROBE_BYTES: usize = 8192;
/// chardet confidence below this means detection is flagged as unreliable.
const ENCODING_CONFIDENCE_THRESHOLD: f32 = 0.6;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePayload {
    pub path: String,
    pub content: String,
    pub file_name: String,
    pub encoding: String,
    pub line_ending: String,
    /// True when the file opened in read-only preview mode (size > 50 MB).
    #[serde(default)]
    pub preview: bool,
    /// False when encoding detection had low confidence.
    #[serde(default = "default_confident")]
    pub encoding_confident: bool,
}

fn default_confident() -> bool {
    true
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
            format!("Invalid file path: {e}")
        }
    })?;
    let meta = std::fs::metadata(&canonical).map_err(|e| format!("Failed to stat path: {e}"))?;
    if !meta.is_file() {
        return Err("Path is not a regular file".to_string());
    }
    Ok(canonical)
}

pub fn detect_encoding(bytes: &[u8]) -> (&'static Encoding, bool) {
    // Fast paths (BOM / valid UTF-8) are always confident.
    if bytes.is_empty() {
        return (encoding_rs::UTF_8, true);
    }
    if bytes.starts_with(b"\xEF\xBB\xBF") {
        return (encoding_rs::UTF_8, true);
    }
    // UTF-16 BOMs take precedence over the chardet fallback.
    if bytes.starts_with(b"\xFF\xFE") {
        return (encoding_rs::UTF_16LE, true);
    }
    if bytes.starts_with(b"\xFE\xFF") {
        return (encoding_rs::UTF_16BE, true);
    }
    if std::str::from_utf8(bytes).is_ok() {
        return (encoding_rs::UTF_8, true);
    }
    let (enc, confidence, _has_bom) = chardet::detect(bytes);
    let mapped = match enc.as_str() {
        "UTF-8" | "ascii" | "ASCII" => encoding_rs::UTF_8,
        "UTF-16LE" => encoding_rs::UTF_16LE,
        "UTF-16BE" => encoding_rs::UTF_16BE,
        "windows-1252" | "ISO-8859-1" | "iso-8859-1" | "ISO-8859-15" => encoding_rs::WINDOWS_1252,
        _ => encoding_rs::UTF_8,
    };
    (mapped, confidence >= ENCODING_CONFIDENCE_THRESHOLD)
}

pub fn detect_line_ending(content: &str) -> &'static str {
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

/// Append the default extension when the chosen name has none.
/// Hidden files (leading dot) are kept as-is.
pub fn ensure_extension(path_str: &str, default_ext: &str) -> String {
    let path = PathBuf::from(path_str);
    let parent = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    if file_name.is_empty() || file_name.starts_with('.') {
        return path_str.to_string();
    }

    let visible = file_name.trim_start_matches('.');
    if !visible.contains('.') {
        let new_name = format!("{file_name}.{default_ext}");
        return parent.join(new_name).to_string_lossy().replace('\\', "/");
    }

    path_str.to_string()
}

/// `encoding_rs` has no UTF-16 encoder (its encoders always emit UTF-8), so
/// UTF-16 byte sequences are produced manually from the string's code units.
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

pub fn encode_content(content: &str, line_ending: &str, encoding: &str) -> Vec<u8> {
    // "LF" must normalize too - otherwise a CRLF-loaded buffer stays CRLF
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
        }
        "UTF-16BE" => {
            let mut out = vec![0xFE, 0xFF];
            out.extend_from_slice(&utf16_bytes(&normalized, true));
            out
        }
        "windows-1252" | "Windows-1252" => encoding_rs::WINDOWS_1252.encode(&normalized).0.to_vec(),
        _ => normalized.as_bytes().to_vec(),
    }
}

pub fn normalize_line_endings(s: &str, target: &str) -> String {
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

/// Read a file, detecting encoding, line endings and binary content.
pub fn read_file(path: &str) -> Result<FilePayload, String> {
    let canonical = validate_path(path)?;

    let metadata =
        std::fs::metadata(&canonical).map_err(|e| format!("Failed to stat file: {e}"))?;
    let size = metadata.len();
    if size > HARD_LIMIT_BYTES {
        return Err(format!(
            "File too large: {:.1} MB (limit: {:.0} MB)",
            size as f64 / 1_048_576.0,
            HARD_LIMIT_BYTES as f64 / 1_048_576.0,
        ));
    }

    let bytes = std::fs::read(&canonical).map_err(|e| format!("Failed to read file: {e}"))?;

    // UTF-16 text is full of NUL bytes - that is structural, not binary.
    let has_utf16_bom = bytes.starts_with(b"\xFF\xFE") || bytes.starts_with(b"\xFE\xFF");
    if is_binary(&bytes) && !has_utf16_bom {
        return Err("Refusing to open: file appears to be binary".to_string());
    }

    let (encoding, confident) = detect_encoding(&bytes);
    Ok(build_payload(&canonical, &bytes, encoding, confident, size))
}

/// Re-read a file with an explicit encoding (low-confidence override).
/// Kept for parity with the Tauri command of the same name.
#[allow(dead_code)]
pub fn read_file_with_encoding(path: &str, encoding: &str) -> Result<FilePayload, String> {
    let canonical = validate_path(path)?;
    let enc: &'static Encoding = match encoding {
        "UTF-8" => encoding_rs::UTF_8,
        "UTF-16LE" => encoding_rs::UTF_16LE,
        "UTF-16BE" => encoding_rs::UTF_16BE,
        "windows-1252" | "Windows-1252" => encoding_rs::WINDOWS_1252,
        _ => return Err(format!("Unknown encoding: {encoding}")),
    };
    let bytes = std::fs::read(&canonical).map_err(|e| format!("Failed to read file: {e}"))?;
    let size = bytes.len() as u64;
    Ok(build_payload(&canonical, &bytes, enc, true, size))
}

fn build_payload(
    canonical: &Path,
    bytes: &[u8],
    encoding: &'static Encoding,
    encoding_confident: bool,
    size: u64,
) -> FilePayload {
    let encoding_name = encoding.name().to_string();
    let (content, _enc, _had_errors) = encoding.decode(bytes);
    let file_name = canonical
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "untitled".to_string());

    FilePayload {
        path: canonical.to_string_lossy().to_string(),
        content: content.to_string(),
        file_name,
        encoding: encoding_name,
        line_ending: detect_line_ending(&content).to_string(),
        preview: size > PREVIEW_LIMIT_BYTES,
        encoding_confident,
    }
}

/// Atomic save: write to a temp file in the same directory, then rename over
/// the target (mirrors the Tauri implementation, chmod/EXDEV fallbacks aside).
pub fn write_atomic(path: &str, data: &[u8]) -> Result<(), String> {
    let target = PathBuf::from(path);
    let parent = target
        .parent()
        .ok_or_else(|| "Invalid save path".to_string())?;
    std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {e}"))?;

    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| format!("Failed to create temp file: {e}"))?;
    temp.write_all(data)
        .map_err(|e| format!("Failed to write temp file: {e}"))?;
    temp.flush()
        .map_err(|e| format!("Failed to flush temp file: {e}"))?;
    temp.persist(&target)
        .map_err(|e| format!("Failed to replace target file: {}", e.error))?;
    Ok(())
}

/// Save content to disk preserving encoding and line endings.
pub fn save_file(
    path: &str,
    content: &str,
    line_ending: &str,
    encoding: &str,
) -> Result<(), String> {
    let data = encode_content(content, line_ending, encoding);
    write_atomic(path, &data)
}

// ---------------------------------------------------------------------------
// persistence helpers (recent files / settings) - JSON under the app data dir
// ---------------------------------------------------------------------------

pub fn app_data_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(std::env::temp_dir);
    base.join("devnote")
}

pub fn read_json<T: for<'de> Deserialize<'de>>(file_name: &str) -> Option<T> {
    let path = app_data_dir().join(file_name);
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn write_json<T: Serialize>(file_name: &str, value: &T) -> Result<(), String> {
    let dir = app_data_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create data dir: {e}"))?;
    let data = serde_json::to_string_pretty(value)
        .map_err(|e| format!("Failed to serialize data: {e}"))?;
    std::fs::write(dir.join(file_name), data).map_err(|e| format!("Failed to write data: {e}"))
}

/// Language label for the status bar (extension map from `detect-lang.ts`).
pub fn language_for(path: Option<&Path>) -> &'static str {
    let Some(path) = path else {
        return "Plain Text";
    };
    match path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
        .as_str()
    {
        "rs" => "Rust",
        "ts" | "tsx" => "TypeScript",
        "js" | "jsx" | "mjs" | "cjs" => "JavaScript",
        "py" => "Python",
        "html" | "htm" => "HTML",
        "css" => "CSS",
        "md" | "markdown" => "Markdown",
        "json" => "JSON",
        "sql" => "SQL",
        "xml" => "XML",
        "vue" => "Vue",
        "svelte" => "Svelte",
        "toml" => "TOML",
        "yaml" | "yml" => "YAML",
        "sh" | "bash" | "zsh" | "fish" => "Shell",
        "go" => "Go",
        "rb" => "Ruby",
        "c" => "C",
        "cpp" | "cc" | "cxx" | "h" | "hpp" => "C++",
        "java" => "Java",
        "php" => "PHP",
        _ => "Plain Text",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_line_ending_prefers_crlf() {
        assert_eq!(detect_line_ending("a\r\nb\r\nc"), "CRLF");
        assert_eq!(detect_line_ending("a\nb\nc"), "LF");
        assert_eq!(detect_line_ending("a\rb\rc"), "CR");
    }

    #[test]
    fn detect_encoding_prefers_utf16_boms_over_chardet() {
        let bytes = [0xFF, 0xFE, b'a', 0x00];
        let (enc, confident) = detect_encoding(&bytes);
        assert_eq!(enc.name(), "UTF-16LE");
        assert!(confident);
    }

    #[test]
    fn encode_content_emits_utf16_boms() {
        let out = encode_content("hi", "LF", "UTF-16LE");
        assert_eq!(out[..2], [0xFF, 0xFE]);
        assert_eq!(out.len(), 2 + 4);
    }

    #[test]
    fn normalize_line_endings_crlf() {
        assert_eq!(
            normalize_line_endings("a\nb\rc\r\nd", "\r\n"),
            "a\r\nb\r\nc\r\nd"
        );
    }

    #[test]
    fn ensure_extension_appends_only_when_missing() {
        assert_eq!(ensure_extension("/tmp/notes", "txt"), "/tmp/notes.txt");
        assert_eq!(ensure_extension("/tmp/notes.md", "txt"), "/tmp/notes.md");
        assert_eq!(ensure_extension("/tmp/.env", "txt"), "/tmp/.env");
    }

    #[test]
    fn atomic_save_round_trips_and_leaves_no_temp_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("note.txt");
        let target_str = target.to_string_lossy().to_string();

        save_file(&target_str, "hello\nworld", "LF", "UTF-8").expect("save");
        assert_eq!(
            std::fs::read_to_string(&target).expect("read"),
            "hello\nworld"
        );

        // CRLF round trip
        save_file(&target_str, "a\nb", "CRLF", "UTF-8").expect("save");
        assert_eq!(std::fs::read_to_string(&target).expect("read"), "a\r\nb");

        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .expect("read_dir")
            .filter_map(Result::ok)
            .filter(|e| e.file_name() != "note.txt")
            .collect();
        assert!(
            leftovers.is_empty(),
            "temp files left behind: {leftovers:?}"
        );
    }

    #[test]
    fn is_binary_detects_nul_byte() {
        assert!(is_binary(b"abc\0def"));
        assert!(!is_binary(b"plain text"));
    }
}
