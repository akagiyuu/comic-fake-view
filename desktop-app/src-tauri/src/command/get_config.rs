use comic_fake_view_core::config::Config;

#[tauri::command]
pub async fn get_config(_: tauri::AppHandle) -> Config {
    Config::load()
}
