use std::path::PathBuf;
use std::sync::Mutex;

pub struct RecentFilesState {
  pub files: Mutex<Vec<String>>,
  persistence_path: PathBuf,
}

impl RecentFilesState {
  pub const MAX_ENTRIES: usize = 10;

  pub fn new(app_data_dir: PathBuf) -> Self {
    let persistence_path = app_data_dir.join("recent_files.json");
    let files = Self::load_from_disk(&persistence_path).unwrap_or_default();
    Self {
      files: Mutex::new(files),
      persistence_path,
    }
  }

  fn load_from_disk(path: &std::path::Path) -> Option<Vec<String>> {
    let json = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&json).ok()
  }

  pub fn persist(&self) -> Result<(), String> {
    let files = self.files.lock().map_err(|e| e.to_string())?;
    let json = serde_json::to_string(&*files).map_err(|e| e.to_string())?;
    if let Some(parent) = self.persistence_path.parent() {
      std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&self.persistence_path, json).map_err(|e| e.to_string())
  }
}
