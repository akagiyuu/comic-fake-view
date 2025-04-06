use comic_fake_view_core::config::Config;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{mpsc, watch, Mutex};

#[tauri::command]
pub async fn run(app_handle: AppHandle) {
    let config = Config::load();
    let cancellation = app_handle.state::<watch::Receiver<bool>>().inner().clone();
    app_handle
        .state::<Mutex<watch::Sender<bool>>>()
        .lock()
        .await
        .send_replace(false);

    let (progress_notifier, mut progress_receiver) = mpsc::unbounded_channel();

    let app_handle_clone = app_handle.clone();
    tokio::spawn(async move {
        while let Some((event, data)) = progress_receiver.recv().await {
            app_handle_clone.emit(event, data).unwrap();
        }
    });

    if let Err(error) = comic_fake_view_core::run(progress_notifier, cancellation, &config).await {
        tracing::error!("{:?}", error)
    }

    app_handle.emit("completed", ()).unwrap();
}
