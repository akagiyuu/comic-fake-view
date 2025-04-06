mod config;

use std::{sync::Arc, time::Duration};

use color_eyre::Result;
use database::job;
use tokio::{
    sync::{mpsc, watch},
    task::JoinSet,
    time::sleep,
};

pub use crate::config::Config;

pub async fn run(
    progress_notifier: mpsc::UnboundedSender<(&'static str, String)>,
    mut cancellation: watch::Receiver<bool>,
    automation_config: Arc<Config>,
    browser_config: &browser::Config,
) -> Result<()> {
    let pool = database::init().await?;

    let job_receiver = job::all(&pool).await?;
    progress_notifier.send(("total_jobs", job_receiver.len().to_string()))?;

    let mut browser = browser::init(browser_config).await?;

    let mut join_set = JoinSet::<Result<()>>::new();
    for _ in 0..automation_config.tab_count {
        let progress_notifier = progress_notifier.clone();
        let receiver = job_receiver.clone();

        let page = browser::new_blank_tab(&browser).await?;
        let pool = pool.clone();
        let config = automation_config.clone();
        join_set.spawn(async move {
            let page_ref = &page;

            while let Ok(chapter_url) = receiver.recv_timeout(Duration::from_secs(10)) {
                let res = {
                    browser::read(&chapter_url, page_ref, config.max_retries).await?;

                    job::done(&chapter_url, &pool).await?;
                    Ok::<(), color_eyre::eyre::Error>(())
                };
                if let Err(error) = res {
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
