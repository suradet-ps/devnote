mod commands;
mod state;

use state::recent::RecentFilesState;
use std::sync::Mutex;
use tauri::Manager;
use tauri::menu::{MenuBuilder, SubmenuBuilder, MenuItemBuilder};

fn build_menu(app: &tauri::AppHandle) -> tauri::menu::Menu<tauri::Wry> {
    let file_menu = SubmenuBuilder::new(app, "File")
        .item(&MenuItemBuilder::new("New Tab").id("menu-new-tab").accelerator("CmdOrCtrl+N").build(app).unwrap())
        .item(&MenuItemBuilder::new("Open...").id("menu-open").accelerator("CmdOrCtrl+O").build(app).unwrap())
        .separator()
        .item(&MenuItemBuilder::new("Save").id("menu-save").accelerator("CmdOrCtrl+S").build(app).unwrap())
        .item(&MenuItemBuilder::new("Save As...").id("menu-save-as").accelerator("CmdOrCtrl+Shift+S").build(app).unwrap())
        .separator()
        .item(&MenuItemBuilder::new("Close Tab").id("menu-close-tab").accelerator("CmdOrCtrl+W").build(app).unwrap())
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
        .build()
        .unwrap();

    let view_menu = SubmenuBuilder::new(app, "View")
        .item(&MenuItemBuilder::new("Find...").id("menu-find").accelerator("CmdOrCtrl+F").build(app).unwrap())
        .item(&MenuItemBuilder::new("Find & Replace...").id("menu-find-replace").accelerator("CmdOrCtrl+H").build(app).unwrap())
        .separator()
        .item(&MenuItemBuilder::new("Zoom In").id("menu-zoom-in").accelerator("CmdOrCtrl+=").build(app).unwrap())
        .item(&MenuItemBuilder::new("Zoom Out").id("menu-zoom-out").accelerator("CmdOrCtrl+-").build(app).unwrap())
        .item(&MenuItemBuilder::new("Reset Zoom").id("menu-zoom-reset").accelerator("CmdOrCtrl+0").build(app).unwrap())
        .separator()
        .item(&MenuItemBuilder::new("Toggle Word Wrap").id("menu-word-wrap").accelerator("Alt+Z").build(app).unwrap())
        .build()
        .unwrap();

    MenuBuilder::new(app)
        .item(&file_menu)
        .item(&edit_menu)
        .item(&view_menu)
        .build()
        .unwrap()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(RecentFilesState(Mutex::new(Vec::new())))
        .invoke_handler(tauri::generate_handler![
            commands::file::open_file,
            commands::file::read_file,
            commands::file::save_file,
            commands::file::save_file_as,
            commands::file::add_recent_file,
            commands::file::get_recent_files,
            commands::file::remove_recent_file,
            commands::window::set_window_title,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::Ready = event {
            if let Some(window) = app_handle.get_webview_window("main") {
                let menu = build_menu(app_handle);
                window.set_menu(menu).ok();
            }
        }

        if let tauri::RunEvent::MenuEvent(ref menu_event) = event {
            if let Some(window) = app_handle.get_webview_window("main") {
                use tauri::Emitter;
                let event_id = menu_event.id().as_ref();
                match event_id {
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
                    "menu-zoom-in" => { window.emit("menu-zoom-in", ()).ok(); }
                    "menu-zoom-out" => { window.emit("menu-zoom-out", ()).ok(); }
                    "menu-zoom-reset" => { window.emit("menu-zoom-reset", ()).ok(); }
                    "menu-word-wrap" => { window.emit("menu-word-wrap", ()).ok(); }
                    _ => {}
                }
            }
        }

        if let tauri::RunEvent::Opened { urls } = event {
            if let Some(window) = app_handle.get_webview_window("main") {
                use tauri::Emitter;
                let paths: Vec<String> = urls
                    .iter()
                    .filter_map(|u| u.to_file_path().ok().and_then(|p| p.to_str().map(String::from)))
                    .collect();
                if !paths.is_empty() {
                    window.emit("file-opened", paths).ok();
                }
            }
        }
    });
}
