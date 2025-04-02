use crate::config::Config;

#[tauri::command]
pub async fn set_config(app_handle: tauri::AppHandle, config: Config) {
    config.save().await;
}
