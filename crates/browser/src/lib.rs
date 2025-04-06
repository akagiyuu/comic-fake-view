pub mod config;

use std::time::Duration;

use backon::{ExponentialBuilder, Retryable};
use chromiumoxide::{Browser, Page, error::CdpError};
use color_eyre::eyre::{Context, Result};
use config::Config;
use futures::StreamExt;
use tokio::time::sleep;

pub async fn init(config: &Config) -> Result<Browser> {
    let (browser, mut handler) = Browser::launch(config.try_into()?).await.unwrap();

    tokio::spawn(async move {
        loop {
            let _ = handler.next().await;
        }
    });

    Ok(browser)
}

pub async fn new_blank_tab(browser: &Browser) -> Result<Page> {
    let page = browser.new_page("about:blank").await?;

    Ok(page)
}

pub async fn read(chapter_url: &str, page: &Page, max_retries: usize) -> Result<()> {
    let read_chapter = || async move {
        page.goto(chapter_url).await?;

        Ok(())
    };

    read_chapter
        .retry(ExponentialBuilder::default().with_max_times(max_retries))
        .sleep(sleep)
        .when(|error| {
            !matches!(
                error,
                CdpError::ChannelSendError(chromiumoxide::error::ChannelError::Send(send_error)) if send_error.is_disconnected()
            )
        })
        .notify(|err, dur: Duration| {
            tracing::warn!("retrying {:?} after {:?}", err, dur);
        })
        .await
        .with_context(|| format!("Failed to read chapter {}", chapter_url))
}
