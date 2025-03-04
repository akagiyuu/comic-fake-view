use crate::config::Config;
use futures::lock::Mutex;
use tauri::Manager;

#[tauri::command]
pub async fn set_config(app_handle: tauri::AppHandle, config: Config) {
    *app_handle.state::<Mutex<Config>>().lock().await = config;
}
