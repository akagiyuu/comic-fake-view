use std::sync::Arc;

use automation::Message;
use futures::StreamExt;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{watch, Mutex};

use crate::config::Config;

#[tauri::command]
pub async fn run(app_handle: AppHandle) {
    let config = Config::load().unwrap();
    let cancellation = app_handle.state::<watch::Receiver<bool>>().inner().clone();
    app_handle
        .state::<Mutex<watch::Sender<bool>>>()
        .lock()
        .await
        .send_replace(false);

    let task = automation::run(
        Some(cancellation),
        Arc::new(config.automation_config),
        &config.browser_config,
    )
    .await;
    tokio::pin!(task);

    while let Some(message) = task.next().await {
        match message {
            Ok(Message::JobCount(count)) => app_handle.emit("total_jobs", count).unwrap(),
            Ok(Message::CompleteJob) => app_handle.emit("complete", ()).unwrap(),
            Err(error) => app_handle.emit("error", error.to_string()).unwrap(),
        }
    }

    app_handle.emit("completed", ()).unwrap();
}
