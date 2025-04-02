pub mod command;
pub mod config;

use futures::lock::Mutex;
use tauri::Manager;

use crate::config::Config;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![command::set_config, command::get_config, command::run])
        .setup(|app| {
            app.manage(Mutex::new(Config::default()));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
