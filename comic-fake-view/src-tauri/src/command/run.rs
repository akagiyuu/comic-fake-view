use std::{sync::Arc, time::Duration};

use tauri::{AppHandle, Emitter};
use tokio::{sync::RwLock, task::JoinSet};

use crate::{
    browser,
    config::Config,
    database::{self, job},
};

#[tauri::command]
pub async fn run(app_handle: AppHandle) {
    let config = Arc::new(Config::load());
    let pool = database::init().await.unwrap();

    let job_receiver = job::all(&pool).await.unwrap();
    app_handle.emit("total_jobs", job_receiver.len()).unwrap();

    let browser = Arc::new(RwLock::new(browser::init(&config).await.unwrap()));

    let mut join_set = JoinSet::new();
    for _ in 0..config.tab_count {
        let app_handle = app_handle.clone();
        let receiver = job_receiver.clone();

        let browser_ref = browser.clone();
        let config = config.clone();
        let pool = pool.clone();
        join_set.spawn(async move {
            let browser_read = browser_ref.read().await;
            let page = browser::new_blank_tab(&browser_read).await.unwrap();
            let page_ref = &page;

            while let Ok(chapter_url) = receiver.recv_timeout(Duration::from_secs(10)) {
                if let Err(error) = browser::read(&chapter_url, page_ref, &pool, &config).await {
                    tracing::error!("{}", error);
                    app_handle.emit("error", error.to_string()).unwrap();
                    break;
                }

                app_handle.emit("complete", ()).unwrap();
            }

            let _ = page.close().await;
        });
    }

    while join_set.join_next().await.is_some() {}

    database::clean(&pool).await.unwrap();

    tokio::spawn(async move {
        browser.write().await.close().await.unwrap();
    });
    tracing::info!("Finish");

    app_handle.emit("completed", ()).unwrap();
}
