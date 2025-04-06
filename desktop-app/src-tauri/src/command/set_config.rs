use crate::config::Config;

#[tauri::command]
pub async fn set_config(_: tauri::AppHandle, config: Config) {
    config.save().await.unwrap();
}
