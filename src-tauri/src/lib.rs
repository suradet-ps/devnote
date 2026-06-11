mod commands;
mod state;

use state::recent::RecentFilesState;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
