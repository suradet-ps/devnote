use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryEntry {
  pub file_name: String,
  pub content: String,
  pub path: Option<String>,
  pub saved_at: String,
}

#[derive(Debug)]
pub struct RecoveryState {
  pub dir: PathBuf,
  pub entries: Mutex<Vec<RecoveryEntry>>,
}

impl RecoveryState {
  pub fn new(dir: PathBuf) -> Self {
    Self {
      dir,
      entries: Mutex::new(Vec::new()),
    }
  }
}
