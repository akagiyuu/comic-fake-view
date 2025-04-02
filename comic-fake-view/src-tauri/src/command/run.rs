use std::{sync::Arc, time::Duration};

use anyhow::Result;
use tauri::{AppHandle, Emitter, Manager};
use tokio::{
    sync::{watch, Mutex, RwLock},
    task::JoinSet,
};

use crate::{
    browser,
    config::Config,
    database::{self, job},
};

async fn _run(app_handle: AppHandle) -> Result<()> {
    let config = Arc::new(Config::load());
    let pool = database::init().await?;

    let job_receiver = job::all(&pool).await?;
    app_handle.emit("total_jobs", job_receiver.len())?;

    let browser = Arc::new(RwLock::new(browser::init(&config).await?));
    app_handle
        .state::<Mutex<watch::Sender<bool>>>()
        .lock()
        .await
        .send_replace(false);

    let mut join_set = JoinSet::<Result<()>>::new();
    for _ in 0..config.tab_count {
        let app_handle = app_handle.clone();
        let receiver = job_receiver.clone();

        let browser_ref = browser.clone();
        let config = config.clone();
        let pool = pool.clone();
        join_set.spawn(async move {
            let browser_read = browser_ref.read().await;
            let page = browser::new_blank_tab(&browser_read).await?;
            let page_ref = &page;

            let is_stopped = app_handle.state::<watch::Receiver<bool>>();
            while let Ok(chapter_url) = receiver.recv_timeout(Duration::from_secs(10)) {
                if *is_stopped.borrow() {
                    break;
                }
                if let Err(error) = browser::read(&chapter_url, page_ref, &pool, &config).await {
                    tracing::error!("{}", error);
                    app_handle.emit("error", error.to_string())?;
                    break;
                }

                app_handle.emit("complete", ())?;
            }

            page.close().await?;

            Ok(())
        });
    }

    while let Some(res) = join_set.join_next().await {
        if let Err(error) = res {
            tracing::error!("{:?}", error);
        }
    }

    database::clean(&pool).await?;

    browser.write().await.close().await?;
    tracing::info!("Finish");

    app_handle.emit("completed", ())?;

    Ok(())
}

#[tauri::command]
pub async fn run(app_handle: AppHandle) {
    if let Err(error) = _run(app_handle).await {
        tracing::error!("{:?}", error)
    }
}
