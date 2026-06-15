#![allow(unexpected_cfgs)]

mod commands;
#[cfg(target_os = "macos")]
mod macos_events;
mod state;

use state::PendingFilesState;
use state::recent::RecentFilesState;
use state::recovery::RecoveryState;
use tauri::Manager;
use tauri::menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder, SubmenuBuilder};

fn init_logging(app_data_dir: &std::path::Path) {
    let log_dir = app_data_dir.join("logs");
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        eprintln!("text-rs: failed to create log dir {:?}: {}", log_dir, e);
        return;
    }
    let log_file = log_dir.join("text-rs.log");
    match std::fs::File::create(&log_file) {
        Ok(file) => {
            let _ = simplelog::CombinedLogger::init(vec![simplelog::WriteLogger::new(
                simplelog::LevelFilter::Info,
                simplelog::Config::default(),
                file,
            )]);
        }
        Err(e) => {
            eprintln!("text-rs: failed to open log file {:?}: {}", log_file, e);
        }
    }
}

fn build_menu(app: &tauri::AppHandle) -> tauri::menu::Menu<tauri::Wry> {
    // Keyboard accelerators are NOT set on menu items on purpose.
    // On macOS, a registered accelerator makes the OS consume the key
    // event, so the renderer's keydown handler never sees it. Because
    // Tauri events emitted from menu clicks are also subject to
    // cross-platform reliability issues, we handle ALL keyboard
    // shortcuts via a document-level keydown handler in
    // src/routes/+page.svelte. The menu items still emit Tauri events
    // when clicked (so the user can drive the app from the menu bar).
    let open_recent = SubmenuBuilder::new(app, "Open Recent")
        .item(
            &MenuItemBuilder::new("(No Recent Files)")
                .enabled(false)
                .build(app)
                .unwrap(),
        )
        .build()
        .unwrap();

    let file_menu = SubmenuBuilder::new(app, "File")
        .item(
            &MenuItemBuilder::new("New Tab")
                .id("menu-new-tab")
                .build(app)
                .unwrap(),
        )
        .item(
            &MenuItemBuilder::new("Open...")
                .id("menu-open")
                .build(app)
                .unwrap(),
        )
        .item(&open_recent)
        .separator()
        .item(
            &MenuItemBuilder::new("Save")
                .id("menu-save")
                .build(app)
                .unwrap(),
        )
        .item(
            &MenuItemBuilder::new("Save As...")
                .id("menu-save-as")
                .build(app)
                .unwrap(),
        )
        .separator()
        .item(
            &MenuItemBuilder::new("Close Tab")
                .id("menu-close-tab")
                .build(app)
                .unwrap(),
        )
        .separator()
        .quit()
        .build()
        .unwrap();

    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .item(
            &MenuItemBuilder::new("Undo")
                .id("menu-undo")
                .build(app)
                .unwrap(),
        )
        .item(
            &MenuItemBuilder::new("Redo")
                .id("menu-redo")
                .build(app)
                .unwrap(),
        )
        .separator()
        .item(
            &MenuItemBuilder::new("Cut")
                .id("menu-cut")
                .build(app)
                .unwrap(),
        )
        .item(
            &MenuItemBuilder::new("Copy")
                .id("menu-copy")
                .build(app)
                .unwrap(),
        )
        .item(
            &MenuItemBuilder::new("Paste")
                .id("menu-paste")
                .build(app)
                .unwrap(),
        )
        .item(
            &MenuItemBuilder::new("Select All")
                .id("menu-select-all")
                .build(app)
                .unwrap(),
        )
        .separator()
        .item(
            &MenuItemBuilder::new("Find...")
                .id("menu-find")
                .build(app)
                .unwrap(),
        )
        .item(
            &MenuItemBuilder::new("Find & Replace...")
                .id("menu-find-replace")
                .build(app)
                .unwrap(),
        )
        .item(
            &MenuItemBuilder::new("Go to Line...")
                .id("menu-go-to-line")
                .build(app)
                .unwrap(),
        )
        .build()
        .unwrap();

    let word_wrap_item = CheckMenuItemBuilder::new("Word Wrap")
        .id("menu-word-wrap")
        .build(app)
        .unwrap();

    let status_bar_item = CheckMenuItemBuilder::new("Status Bar")
        .id("menu-status-bar")
        .build(app)
        .unwrap();

    let view_menu = SubmenuBuilder::new(app, "View")
        .item(&word_wrap_item)
        .item(&status_bar_item)
        .separator()
        .item(
            &MenuItemBuilder::new("Zoom In")
                .id("menu-zoom-in")
                .build(app)
                .unwrap(),
        )
        .item(
            &MenuItemBuilder::new("Zoom Out")
                .id("menu-zoom-out")
                .build(app)
                .unwrap(),
        )
        .item(
            &MenuItemBuilder::new("Reset Zoom")
                .id("menu-zoom-reset")
                .build(app)
                .unwrap(),
        )
        .build()
        .unwrap();

    let window_menu = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .build()
        .unwrap();

    let help_menu = SubmenuBuilder::new(app, "Help")
        .item(
            &MenuItemBuilder::new("About Text RS")
                .id("menu-about")
                .build(app)
                .unwrap(),
        )
        .build()
        .unwrap();

    MenuBuilder::new(app)
        .item(&file_menu)
        .item(&edit_menu)
        .item(&view_menu)
        .item(&window_menu)
        .item(&help_menu)
        .build()
        .unwrap()
}

fn handle_menu_event(app: &tauri::AppHandle, menu_id: &str) {
    use tauri::Emitter;
    log::info!("[menu] event fired: {}", menu_id);
    let Some(window) = app.get_webview_window("main") else {
        log::error!("[menu] main window not found");
        return;
    };
    match menu_id {
        "menu-new-tab" => {
            window.emit("menu-new-tab", ()).ok();
        }
        "menu-open" => {
            window.emit("menu-open", ()).ok();
        }
        "menu-save" => {
            window.emit("menu-save", ()).ok();
        }
        "menu-save-as" => {
            window.emit("menu-save-as", ()).ok();
        }
        "menu-close-tab" => {
            window.emit("menu-close-tab", ()).ok();
        }
        "menu-undo" => {
            window.emit("menu-undo", ()).ok();
        }
        "menu-redo" => {
            window.emit("menu-redo", ()).ok();
        }
        "menu-cut" => {
            window.emit("menu-cut", ()).ok();
        }
        "menu-copy" => {
            window.emit("menu-copy", ()).ok();
        }
        "menu-paste" => {
            window.emit("menu-paste", ()).ok();
        }
        "menu-select-all" => {
            window.emit("menu-select-all", ()).ok();
        }
        "menu-find" => {
            window.emit("menu-find", ()).ok();
        }
        "menu-find-replace" => {
            window.emit("menu-find-replace", ()).ok();
        }
        "menu-go-to-line" => {
            window.emit("menu-go-to-line", ()).ok();
        }
        "menu-zoom-in" => {
            window.emit("menu-zoom-in", ()).ok();
        }
        "menu-zoom-out" => {
            window.emit("menu-zoom-out", ()).ok();
        }
        "menu-zoom-reset" => {
            window.emit("menu-zoom-reset", ()).ok();
        }
        "menu-word-wrap" => {
            window.emit("menu-word-wrap", ()).ok();
        }
        "menu-status-bar" => {
            window.emit("menu-status-bar", ()).ok();
        }
        "menu-about" => {
            window.emit("menu-about", ()).ok();
        }
        id if id.starts_with("recent-") => {
            let path = id.strip_prefix("recent-").unwrap_or("");
            window.emit("menu-open-recent", path.to_string()).ok();
        }
        _ => {
            log::warn!("[menu] unhandled id: {}", menu_id);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::file::open_file,
            commands::file::read_file,
            commands::file::save_file,
            commands::file::save_file_as,
            commands::file::add_recent_file,
            commands::file::get_recent_files,
            commands::file::remove_recent_file,
            commands::file::check_file_size,
            commands::file::get_pending_files,
            commands::file::frontend_ready,
            commands::window::set_window_title,
            commands::recovery::save_recovery_data,
            commands::recovery::check_recovery_data,
            commands::recovery::clear_recovery_data,
        ])
        .setup(|app| {
            let (pending, pending_arc) = PendingFilesState::new();

            // app_data_dir is required for logging, recovery, and recent-files
            // state. If it fails, we abort startup with a clear error rather
            // than registering a half-initialized state.
            let dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to resolve app_data_dir: {}", e))?;

            init_logging(&dir);

            let recovery_dir = dir.join("recovery");
            app.manage(RecoveryState::new(recovery_dir));

            app.manage(RecentFilesState::new(dir));

            // Capture file paths passed as command-line args (macOS "Open With" / drag-to-icon)
            {
                let args: Vec<String> = std::env::args().skip(1).collect();
                log::info!("[launch] argv (after skip 1): {:?}", args);
                let file_args: Vec<String> = args
                    .iter()
                    .filter(|a| !a.starts_with('-'))
                    .cloned()
                    .collect();
                if !file_args.is_empty() {
                    log::info!("[launch] files from argv: {:?}", file_args);
                    if let Ok(mut paths) = pending_arc.lock() {
                        paths.extend(file_args);
                    }
                } else {
                    log::info!(
                        "[launch] no files from argv (macOS file-association uses Apple Event)"
                    );
                }
            }

            app.manage(pending);

            #[cfg(target_os = "macos")]
            {
                macos_events::init(pending_arc.clone());
                macos_events::macos::capture_launch_file(&pending_arc);
                macos_events::macos::install_handler(pending_arc.clone());
                if let Ok(paths) = pending_arc.lock() {
                    log::info!(
                        "[launch] pending files after Apple Event capture: {:?}",
                        *paths
                    );
                }
            }

            let menu = build_menu(app.handle());
            // Use app.set_menu so the menu is registered with the
            // application (system menu bar on macOS, window menu on
            // Windows/Linux). window.set_menu is window-scoped and can
            // miss some events on macOS.
            match app.set_menu(menu) {
                Ok(_) => log::info!("[menu] application menu set successfully"),
                Err(e) => log::error!("[menu] failed to set menu: {}", e),
            }

            log::info!("text-rs started");
            log::info!("CLI args: {:?}", std::env::args().collect::<Vec<_>>());

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| match event {
        tauri::RunEvent::MenuEvent(ref menu_event) => {
            handle_menu_event(app_handle, menu_event.id().as_ref());
        }
        #[cfg(any(target_os = "macos", target_os = "ios", target_os = "android"))]
        tauri::RunEvent::Opened { urls, .. } => {
            use tauri::Emitter;
            log::info!(
                "[RunEvent::Opened] received {} url(s): {:?}",
                urls.len(),
                urls
            );
            let paths: Vec<String> = urls
                .iter()
                .filter_map(|url| {
                    let p = url.to_file_path().ok();
                    if p.is_none() {
                        log::warn!(
                            "[RunEvent::Opened] could not convert url to file path: {}",
                            url
                        );
                    }
                    p
                })
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            if !paths.is_empty() {
                log::info!("[RunEvent::Opened] resolved file paths: {:?}", paths);
                if let Some(pending) = app_handle.try_state::<PendingFilesState>()
                    && let Ok(mut p) = pending.inner.lock()
                {
                    p.extend(paths.clone());
                    log::info!(
                        "[RunEvent::Opened] added to pending list (now {} total)",
                        p.len()
                    );
                }
                if let Some(window) = app_handle.get_webview_window("main") {
                    log::info!("[RunEvent::Opened] emitting file-opened event to frontend");
                    window.emit("file-opened", paths).ok();
                } else {
                    log::warn!("[RunEvent::Opened] main window not found, event not emitted");
                }
            } else {
                log::warn!("[RunEvent::Opened] no valid file paths from urls");
            }
        }
        tauri::RunEvent::ExitRequested { .. } => {
            log::info!("text-rs exiting");
        }
        _ => {}
    });
}
