mod config;

use std::{sync::Arc, time::Duration};

use async_stream::try_stream;
use color_eyre::Result;
use database::job;
use futures::{Stream, StreamExt};
use tokio::{sync::watch, time::sleep};

pub use crate::config::Config;

pub enum Message {
    CompleteJob,
    JobCount(usize),
}

pub async fn run(
    cancellation: Option<watch::Receiver<bool>>,
    automation_config: Arc<Config>,
    browser_config: &browser::Config,
) -> impl Stream<Item = Result<Message>> {
    try_stream! {
        let pool = database::init().await?;

        let jobs = job::all(&pool).await?;
        yield Message::JobCount(jobs.len());

        let mut browser = browser::init(browser_config).await?;
        let (page_sender, page_receiver) = flume::bounded(automation_config.tab_count);
        for _ in 0..automation_config.tab_count {
            let page = browser::new_blank_tab(&browser).await?;
            page_sender.send(page)?;
        }

        let tasks = futures::stream::iter(jobs).map(|chapter_url| {
            let pool = pool.clone();
            let config = automation_config.clone();
            let page_sender = page_sender.clone();
            let page_receiver = page_receiver.clone();
            let cancellation = cancellation.clone();

            async move {
                if let Some(cancellation) = cancellation {
                    if  *cancellation.borrow() {
                        return Err(color_eyre::eyre::eyre!("cancelled"));
                    }
                }

                let page = page_receiver.recv_async().await?;
                browser::read(&chapter_url, &page, config.max_retries).await?;
                page_sender.send_async(page).await?;

                job::done(&chapter_url, &pool).await?;

                sleep(Duration::from_secs(config.wait_for_navigation)).await;

                Ok::<(), color_eyre::eyre::Error>(())
            }
        })
        .buffer_unordered(automation_config.tab_count);
        tokio::pin!(tasks);

        while let Some(res) = tasks.next().await {
            res?;
            yield Message::CompleteJob;
        }

        database::clean(&pool).await?;

        browser.close().await?;
        tracing::info!("Finish");
    }
}
