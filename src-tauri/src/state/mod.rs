pub mod recent;
pub mod recovery;

use std::sync::{Arc, Mutex};

pub struct PendingFilesState {
  pub inner: Arc<Mutex<Vec<String>>>,
}

impl PendingFilesState {
  pub fn new() -> (Self, Arc<Mutex<Vec<String>>>) {
    let inner = Arc::new(Mutex::new(Vec::new()));
    (
      Self {
        inner: inner.clone(),
      },
      inner,
    )
  }

  pub fn drain(&self) -> Vec<String> {
    self
      .inner
      .lock()
      .map(|mut p| p.drain(..).collect())
      .unwrap_or_default()
  }
}
