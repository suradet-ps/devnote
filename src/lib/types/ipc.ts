/**
 * Frontend ↔ backend IPC contracts.
 *
 * Conventions (Tauri 2):
 * - Command parameter names: camelCase in JS, auto-converted to snake_case in Rust.
 * - Command return values / struct field names: passed through serde with no rename,
 *   so we use snake_case here to match the Rust struct definitions exactly.
 */

/** Returned by `read_file`, `open_file`, `read_file_with_encoding`. */
export interface FilePayload {
  path: string;
  content: string;
  file_name: string;
  encoding: string;
  line_ending: string;
  /** True when opened in read-only preview mode (size > 50 MB). */
  preview?: boolean;
  /** False when encoding detection had low confidence (user may override). */
  encoding_confident?: boolean;
}

/** Returned by `check_recovery_data` and consumed by `save_recovery_data`. */
export interface RecoveryEntry {
  file_name: string;
  content: string;
  path: string | null;
  saved_at: string;
}
