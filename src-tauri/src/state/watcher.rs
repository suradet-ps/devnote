use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use tauri::{Emitter, Manager};

/// Minimum gap between two external-change events for the same file.
pub const DEBOUNCE: Duration = Duration::from_millis(500);
/// Window after our own save during which events are ignored (the atomic
/// rename triggers a Modify event that is not an external change).
pub const SELF_SAVE_SUPPRESS: Duration = Duration::from_secs(1);

/// Global registry of our own saves, keyed by canonical path. Written by
/// `note_self_save` after every successful save (no AppHandle needed, so the
/// save commands stay pure and testable) and read by the watcher callback.
static LAST_SELF_SAVE: OnceLock<Arc<Mutex<HashMap<PathBuf, Instant>>>> = OnceLock::new();

fn self_save_registry() -> &'static Arc<Mutex<HashMap<PathBuf, Instant>>> {
  LAST_SELF_SAVE.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

/** Record a successful save so the resulting fs event is treated as internal. */
pub fn note_self_save(path: &std::path::Path) {
  if let Ok(mut m) = self_save_registry().lock() {
    m.insert(path.to_path_buf(), Instant::now());
  }
}

/// Pure decision helper (testable): should an external-change event for a
/// path be emitted right now?
pub fn should_emit(
  last_event: Option<Instant>,
  last_self_save: Option<Instant>,
  now: Instant,
) -> bool {
  if let Some(t) = last_self_save
    && now.duration_since(t) < SELF_SAVE_SUPPRESS
  {
    return false;
  }
  if let Some(t) = last_event
    && now.duration_since(t) < DEBOUNCE
  {
    return false;
  }
  true
}

/**
 * Watches files that are open in tabs and emits a `file-changed-external`
 * Tauri event when a watched file changes on disk.
 *
 * - Only paths registered via `watch_file` are watched (never auto-watch).
 * - Events are debounced per path.
 * - Events caused by our own saves are suppressed (self-save suppression).
 * - Never auto-reloads — the frontend decides (data-loss risk).
 */
pub struct FileWatcherState {
  pub watcher: Mutex<Option<RecommendedWatcher>>,
  watched: Arc<Mutex<HashSet<PathBuf>>>,
  last_event: Arc<Mutex<HashMap<PathBuf, Instant>>>,
}

impl FileWatcherState {
  pub fn new(app: tauri::AppHandle) -> Self {
    let watched = Arc::new(Mutex::new(HashSet::new()));
    let last_event = Arc::new(Mutex::new(HashMap::new()));

    let (watched_cb, last_event_cb) = (watched.clone(), last_event.clone());
    let app_cb = app.clone();

    let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
      let Ok(event) = res else { return };
      for path in event.paths {
        // Only files we were asked to watch
        if !watched_cb
          .lock()
          .map(|w| w.contains(&path))
          .unwrap_or(false)
        {
          continue;
        }
        let now = Instant::now();
        let self_save = self_save_registry()
          .lock()
          .ok()
          .and_then(|m| m.get(&path).copied());
        let last = last_event_cb
          .lock()
          .ok()
          .and_then(|m| m.get(&path).copied());
        if !should_emit(last, self_save, now) {
          continue;
        }
        if let Ok(mut m) = last_event_cb.lock() {
          m.insert(path.clone(), now);
        }
        if let Some(window) = app_cb.get_webview_window("main") {
          let _ = window.emit("file-changed-external", path.to_string_lossy().to_string());
        }
      }
    });

    let watcher = match watcher {
      Ok(w) => Some(w),
      Err(e) => {
        log::warn!("[watcher] failed to create file watcher: {}", e);
        None
      },
    };

    Self {
      watcher: Mutex::new(watcher),
      watched,
      last_event,
    }
  }

  pub fn add(&self, path: &std::path::Path) -> Result<(), String> {
    self
      .watched
      .lock()
      .map_err(|e| e.to_string())?
      .insert(path.to_path_buf());
    if let Some(w) = self.watcher.lock().map_err(|e| e.to_string())?.as_mut() {
      w.watch(path, RecursiveMode::NonRecursive)
        .map_err(|e| format!("Failed to watch file: {}", e))?;
    }
    Ok(())
  }

  pub fn remove(&self, path: &std::path::Path) {
    if let Ok(mut w) = self.watched.lock() {
      w.remove(path);
    }
    if let Ok(mut w) = self.watcher.lock()
      && let Some(w) = w.as_mut()
    {
      let _ = w.unwatch(path);
    }
    if let Ok(mut m) = self.last_event.lock() {
      m.remove(path);
    }
    if let Ok(mut m) = self_save_registry().lock() {
      m.remove(path);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn emits_when_no_history() {
    assert!(should_emit(None, None, Instant::now()));
  }

  #[test]
  fn debounces_recent_events() {
    let now = Instant::now();
    assert!(!should_emit(Some(now), None, now + DEBOUNCE / 2));
    assert!(should_emit(
      Some(now),
      None,
      now + DEBOUNCE + Duration::from_millis(1)
    ));
  }

  #[test]
  fn suppresses_own_saves() {
    let now = Instant::now();
    assert!(!should_emit(None, Some(now), now + SELF_SAVE_SUPPRESS / 2));
    assert!(should_emit(
      None,
      Some(now),
      now + SELF_SAVE_SUPPRESS + Duration::from_millis(1)
    ));
  }

  #[test]
  fn own_save_takes_precedence_over_debounce() {
    let now = Instant::now();
    assert!(!should_emit(
      Some(now),
      Some(now),
      now + Duration::from_millis(10)
    ));
  }

  #[test]
  fn self_save_registry_records_and_expires() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("x.txt");
    note_self_save(&path);
    let now = Instant::now();
    let recorded = self_save_registry().lock().unwrap().get(&path).copied();
    assert!(!should_emit(None, recorded, now));
    // After the suppression window the same recorded save no longer suppresses
    let later = now + SELF_SAVE_SUPPRESS + Duration::from_millis(1);
    assert!(should_emit(None, recorded, later));
  }
}
