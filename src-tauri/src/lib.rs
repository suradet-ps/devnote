mod commands;
mod state;

use state::recent::RecentFilesState;
use state::recovery::RecoveryState;
use std::sync::Mutex;
use tauri::Manager;
use tauri::menu::{MenuBuilder, SubmenuBuilder, MenuItemBuilder, CheckMenuItemBuilder};

fn init_logging(app_data_dir: &std::path::Path) {
    let log_dir = app_data_dir.join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let log_file = log_dir.join("text-rs.log");

    let _ = simplelog::CombinedLogger::init(vec![
        simplelog::WriteLogger::new(
            simplelog::LevelFilter::Info,
            simplelog::Config::default(),
            std::fs::File::create(&log_file).unwrap_or(std::fs::File::create("/dev/null").unwrap()),
        ),
    ]);
}

fn build_menu(app: &tauri::AppHandle) -> tauri::menu::Menu<tauri::Wry> {
    let open_recent = SubmenuBuilder::new(app, "Open Recent")
        .build()
        .unwrap();

    let file_menu = SubmenuBuilder::new(app, "File")
        .item(&MenuItemBuilder::new("New Tab").id("menu-new-tab").accelerator("CmdOrCtrl+N").build(app).unwrap())
        .item(&MenuItemBuilder::new("Open...").id("menu-open").accelerator("CmdOrCtrl+O").build(app).unwrap())
        .item(&open_recent)
        .separator()
        .item(&MenuItemBuilder::new("Save").id("menu-save").accelerator("CmdOrCtrl+S").build(app).unwrap())
        .item(&MenuItemBuilder::new("Save As...").id("menu-save-as").accelerator("CmdOrCtrl+Shift+S").build(app).unwrap())
        .separator()
        .item(&MenuItemBuilder::new("Close Tab").id("menu-close-tab").accelerator("CmdOrCtrl+W").build(app).unwrap())
        .separator()
        .quit()
        .build()
        .unwrap();

    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .item(&MenuItemBuilder::new("Undo").id("menu-undo").accelerator("CmdOrCtrl+Z").build(app).unwrap())
        .item(&MenuItemBuilder::new("Redo").id("menu-redo").accelerator("CmdOrCtrl+Shift+Z").build(app).unwrap())
        .separator()
        .item(&MenuItemBuilder::new("Cut").id("menu-cut").accelerator("CmdOrCtrl+X").build(app).unwrap())
        .item(&MenuItemBuilder::new("Copy").id("menu-copy").accelerator("CmdOrCtrl+C").build(app).unwrap())
        .item(&MenuItemBuilder::new("Paste").id("menu-paste").accelerator("CmdOrCtrl+V").build(app).unwrap())
        .item(&MenuItemBuilder::new("Select All").id("menu-select-all").accelerator("CmdOrCtrl+A").build(app).unwrap())
        .separator()
        .item(&MenuItemBuilder::new("Find...").id("menu-find").accelerator("CmdOrCtrl+F").build(app).unwrap())
        .item(&MenuItemBuilder::new("Find & Replace...").id("menu-find-replace").accelerator("CmdOrCtrl+H").build(app).unwrap())
        .item(&MenuItemBuilder::new("Go to Line...").id("menu-go-to-line").accelerator("CmdOrCtrl+G").build(app).unwrap())
        .build()
        .unwrap();

    let word_wrap_item = CheckMenuItemBuilder::new("Word Wrap")
        .id("menu-word-wrap")
        .accelerator("Alt+Z")
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
        .item(&MenuItemBuilder::new("Zoom In").id("menu-zoom-in").accelerator("CmdOrCtrl+=").build(app).unwrap())
        .item(&MenuItemBuilder::new("Zoom Out").id("menu-zoom-out").accelerator("CmdOrCtrl+-").build(app).unwrap())
        .item(&MenuItemBuilder::new("Reset Zoom").id("menu-zoom-reset").accelerator("CmdOrCtrl+0").build(app).unwrap())
        .build()
        .unwrap();

    let window_menu = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .build()
        .unwrap();

    let help_menu = SubmenuBuilder::new(app, "Help")
        .item(&MenuItemBuilder::new("About text-rs").id("menu-about").build(app).unwrap())
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
    if let Some(window) = app.get_webview_window("main") {
        match menu_id {
            "menu-new-tab" => { window.emit("menu-new-tab", ()).ok(); }
            "menu-open" => { window.emit("menu-open-file", ()).ok(); }
            "menu-save" => { window.emit("menu-save", ()).ok(); }
            "menu-save-as" => { window.emit("menu-save-as", ()).ok(); }
            "menu-close-tab" => { window.emit("menu-close-tab", ()).ok(); }
            "menu-undo" => { window.emit("menu-undo", ()).ok(); }
            "menu-redo" => { window.emit("menu-redo", ()).ok(); }
            "menu-cut" => { window.emit("menu-cut", ()).ok(); }
            "menu-copy" => { window.emit("menu-copy", ()).ok(); }
            "menu-paste" => { window.emit("menu-paste", ()).ok(); }
            "menu-select-all" => { window.emit("menu-select-all", ()).ok(); }
            "menu-find" => { window.emit("menu-find", ()).ok(); }
            "menu-find-replace" => { window.emit("menu-find-replace", ()).ok(); }
            "menu-go-to-line" => { window.emit("menu-go-to-line", ()).ok(); }
            "menu-zoom-in" => { window.emit("menu-zoom-in", ()).ok(); }
            "menu-zoom-out" => { window.emit("menu-zoom-out", ()).ok(); }
            "menu-zoom-reset" => { window.emit("menu-zoom-reset", ()).ok(); }
            "menu-word-wrap" => { window.emit("menu-word-wrap", ()).ok(); }
            "menu-status-bar" => { window.emit("menu-status-bar", ()).ok(); }
            "menu-about" => { window.emit("menu-about", ()).ok(); }
            id if id.starts_with("recent-") => {
                let path = id.strip_prefix("recent-").unwrap_or("");
                window.emit("menu-open-recent", path.to_string()).ok();
            }
            _ => {}
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(RecentFilesState(Mutex::new(Vec::new())))
        .invoke_handler(tauri::generate_handler![
            commands::file::open_file,
            commands::file::read_file,
            commands::file::read_file_with_encoding,
            commands::file::save_file,
            commands::file::save_file_as,
            commands::file::add_recent_file,
            commands::file::get_recent_files,
            commands::file::remove_recent_file,
            commands::file::check_file_size,
            commands::window::set_window_title,
            commands::recovery::save_recovery_data,
            commands::recovery::check_recovery_data,
            commands::recovery::clear_recovery_data,
            commands::recovery::get_app_data_dir,
        ])
        .setup(|app| {
            // Initialize logging
            if let Ok(dir) = app.path().app_data_dir() {
                init_logging(&dir);

                // Initialize recovery state
                let recovery_dir = dir.join("recovery");
                app.manage(RecoveryState::new(recovery_dir));
            }

            let menu = build_menu(&app.handle());
            if let Some(window) = app.get_webview_window("main") {
                window.set_menu(menu).ok();
            }

            log::info!("text-rs started");

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        match event {
            tauri::RunEvent::MenuEvent(ref menu_event) => {
                handle_menu_event(app_handle, menu_event.id().as_ref());
            }
            tauri::RunEvent::Opened { urls } => {
                if let Some(window) = app_handle.get_webview_window("main") {
                    use tauri::Emitter;
                    let paths: Vec<String> = urls
                        .iter()
                        .filter_map(|u| u.to_file_path().ok())
                        .filter_map(|p| p.to_str().map(String::from))
                        .collect();
                    if !paths.is_empty() {
                        window.emit("file-opened", paths).ok();
                    }
                }
            }
            tauri::RunEvent::ExitRequested { .. } => {
                log::info!("text-rs exiting");
            }
            _ => {}
        }
    });
}
