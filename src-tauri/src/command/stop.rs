use tauri::Manager;
use tokio::sync::{watch, Mutex};

#[tauri::command]
pub async fn stop(app_handle: tauri::AppHandle) {
    app_handle
        .state::<Mutex<watch::Sender<bool>>>()
        .lock()
        .await
        .send_replace(true);
}
