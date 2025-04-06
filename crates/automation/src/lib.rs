pub mod config;

use std::time::Duration;

use anyhow::Result;
use database::job;
use tokio::{
    sync::{mpsc, watch},
    task::JoinSet,
    time::sleep,
};

use crate::config::Config;

pub async fn run(
    progress_notifier: mpsc::UnboundedSender<(&'static str, String)>,
    mut cancellation: watch::Receiver<bool>,
    config: &Config,
) -> Result<()> {
    let pool = database::init().await?;

    let job_receiver = job::all(&pool).await?;
    progress_notifier.send(("total_jobs", job_receiver.len().to_string()))?;

    let mut browser = browser::init(&config.browser_config).await?;

    let mut join_set = JoinSet::<Result<()>>::new();
    for _ in 0..config.tab_count {
        let progress_notifier = progress_notifier.clone();
        let receiver = job_receiver.clone();

        let page = browser::new_blank_tab(&browser).await?;
        let config = config.clone();
        let pool = pool.clone();
        join_set.spawn(async move {
            let page_ref = &page;

            while let Ok(chapter_url) = receiver.recv_timeout(Duration::from_secs(10)) {
                if let Err(error) = browser::read(&chapter_url, page_ref, config.max_retries).await
                {
                    progress_notifier.send(("error", error.to_string()))?;
                    break;
                }

                sleep(Duration::from_secs(config.wait_for_navigation)).await;

                progress_notifier.send(("complete", "".to_string()))?;
            }

            page.close().await?;

            Ok(())
        });
    }

    tokio::select! {
        _ = async {
            while let Some(res) = join_set.join_next().await {
                if let Err(e) = res {
                    tracing::error!("task failed: {:?}", e);
                }
            }
        } => tracing::info!("All tasks completed."),
        _ = async {
            while cancellation.changed().await.is_ok() {
                if *cancellation.borrow() {
                    tracing::info!("Received stop signal, aborting all tasks.");
                    break;
                }
            }
        } => tracing::info!("cancellation requested, dropping join_set"),
    }

    database::clean(&pool).await?;

    browser.close().await?;
    tracing::info!("Finish");

    Ok(())
}
