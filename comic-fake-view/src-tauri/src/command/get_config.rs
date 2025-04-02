use crate::config::Config;

#[tauri::command]
pub async fn get_config(_: tauri::AppHandle) -> Config {
    Config::load()
}
