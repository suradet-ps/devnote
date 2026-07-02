use std::sync::{Arc, Mutex, OnceLock};

static PENDING: OnceLock<Arc<Mutex<Vec<String>>>> = OnceLock::new();

pub fn init(pending: Arc<Mutex<Vec<String>>>) {
  let _ = PENDING.set(pending);
}

#[cfg(target_os = "macos")]
pub mod macos {
  use super::*;
  use objc::declare::ClassDecl;
  use objc::runtime::{Class, Object, Sel};
  use objc::{class, msg_send, sel, sel_impl};

  // Four-char code for `keyDirectObject` = '----' = 0x2D2D2D2D.
  // The previous value (0x2D646F63, "-doc") was wrong and caused
  // file-open Apple Events to be silently dropped.
  const KEY_DIRECT_OBJECT: u32 = 0x2D2D2D2D;

  /// Capture files from the current Apple Event at startup.
  /// This handles the case where the app was launched with a file (e.g. "Open With" from Finder).
  pub fn capture_launch_file(pending: &Arc<Mutex<Vec<String>>>) {
    unsafe {
      let em_class = match Class::get("NSAppleEventManager") {
        Some(c) => c,
        None => return,
      };
      let em: *mut Object = msg_send![em_class, sharedAppleEventManager];
      if em.is_null() {
        return;
      }
      let event: *mut Object = msg_send![em, currentAppleEvent];
      if event.is_null() {
        log::info!("No current Apple Event at launch");
        return;
      }

      log::info!("Found current Apple Event at launch");
      let descriptor: *mut Object = msg_send![event, paramDescriptorForKeyword: KEY_DIRECT_OBJECT];
      if descriptor.is_null() {
        log::info!("No DirectObject descriptor in launch event");
        return;
      }

      let file_url: *mut Object = msg_send![descriptor, fileURLValue];
      if file_url.is_null() {
        log::info!("No fileURLValue in descriptor");
        return;
      }

      let path: *mut Object = msg_send![file_url, path];
      if path.is_null() {
        log::info!("No path from fileURLValue");
        return;
      }

      let rust_str: *const std::ffi::c_char = msg_send![path, UTF8String];
      if rust_str.is_null() {
        return;
      }
      let cstr = std::ffi::CStr::from_ptr(rust_str);
      let path_str = cstr.to_string_lossy().into_owned();
      if !path_str.is_empty() {
        log::info!("Captured launch file from Apple Event: {}", path_str);
        if let Ok(mut guard) = pending.lock() {
          guard.push(path_str);
        }
      }
    }
  }

  /// Install a persistent handler for future file open events (e.g. when app is already running).
  pub fn install_handler(pending: Arc<Mutex<Vec<String>>>) {
    unsafe {
      let superclass = Class::get("NSObject").expect("NSObject not found");
      let mut cls = ClassDecl::new("TARsFileOpenHandler", superclass)
        .expect("Failed to declare TARsFileOpenHandler class");

      cls.add_method(
        sel!(handleOpenDocument:withReplyEvent:),
        handle_open_event as extern "C" fn(&Object, Sel, *mut Object, *mut Object),
      );
      cls.register();

      let handler_class =
        Class::get("TARsFileOpenHandler").expect("TARsFileOpenHandler class not found");
      let handler: *mut Object = msg_send![handler_class, new];

      let em_class = Class::get("NSAppleEventManager").expect("NSAppleEventManager not found");
      let em: *mut Object = msg_send![em_class, sharedAppleEventManager];

      let event_class: *mut Object =
        msg_send![class!(NSString), stringWithUTF8String: c"aevt".as_ptr()];
      let event_id: *mut Object =
        msg_send![class!(NSString), stringWithUTF8String: c"odoc".as_ptr()];

      let sel_handler = sel!(handleOpenDocument:withReplyEvent:);
      let _: () = msg_send![em,
          setEventHandler: handler
          andSelector: sel_handler
          forEventClass: event_class
          andEventID: event_id
      ];
      log::info!("NSAppleEventManager handler installed for aevt/odoc");
    }

    // Store the pending reference for the handler to use
    HANDLER_PENDING.lock().unwrap().replace(pending);
  }

  static HANDLER_PENDING: std::sync::Mutex<Option<Arc<Mutex<Vec<String>>>>> =
    std::sync::Mutex::new(None);

  extern "C" fn handle_open_event(
    _self: &Object,
    _sel: Sel,
    event: *mut Object,
    _reply: *mut Object,
  ) {
    unsafe {
      let descriptor: *mut Object = msg_send![event, paramDescriptorForKeyword: KEY_DIRECT_OBJECT];
      if descriptor.is_null() {
        return;
      }
      let file_url: *mut Object = msg_send![descriptor, fileURLValue];
      if file_url.is_null() {
        return;
      }
      let path: *mut Object = msg_send![file_url, path];
      if path.is_null() {
        return;
      }
      let rust_str: *const std::ffi::c_char = msg_send![path, UTF8String];
      if rust_str.is_null() {
        return;
      }
      let cstr = std::ffi::CStr::from_ptr(rust_str);
      let path_str = cstr.to_string_lossy().into_owned();
      if !path_str.is_empty() {
        log::info!("Apple Event captured file: {}", path_str);
        if let Ok(guard) = HANDLER_PENDING.lock()
          && let Some(pending) = guard.as_ref()
          && let Ok(mut p) = pending.lock()
        {
          p.push(path_str);
        }
      }
    }
  }
}
