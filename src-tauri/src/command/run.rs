use std::{sync::Arc, time::Duration};

use anyhow::Result;
use tauri::{AppHandle, Emitter, Manager};
use tokio::{
    sync::{watch, Mutex},
    task::JoinSet,
    time::timeout,
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

    let mut browser = browser::init(&config).await?;

    let mut join_set = JoinSet::<Result<()>>::new();
    for _ in 0..config.tab_count {
        let app_handle = app_handle.clone();
        let receiver = job_receiver.clone();

        let page = browser::new_blank_tab(&browser).await?;
        let config = config.clone();
        let pool = pool.clone();
        join_set.spawn(async move {
            let page_ref = &page;

            while let Ok(chapter_url) = receiver.recv_timeout(Duration::from_secs(10)) {
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
    let mut is_stopped = app_handle.state::<watch::Receiver<bool>>().inner().clone();

    app_handle
        .state::<Mutex<watch::Sender<bool>>>()
        .lock()
        .await
        .send_replace(false);

    loop {
        tokio::select! {
            Some(res) = join_set.join_next() => {
                if let Err(error) = res {
                    tracing::error!("{:?}", error);
                }
            }
            _ = is_stopped.changed() => {
                if *is_stopped.borrow() {
                    tracing::info!("Received stop signal, aborting join_set.");
                    join_set.abort_all();
                    break;
                }
            }
        }
    }

    database::clean(&pool).await?;

    timeout(Duration::from_secs(5), async move {
        browser.close().await;
    })
    .await?;
    tracing::info!("Finish");

    Ok(())
}

#[tauri::command]
pub async fn run(app_handle: AppHandle) {
    if let Err(error) = _run(app_handle.clone()).await {
        tracing::error!("{:?}", error)
    }

    app_handle.emit("completed", ()).unwrap();
}
