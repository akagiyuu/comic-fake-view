use std::time::Duration;

use anyhow::{Context, Result};
use backon::{ExponentialBuilder, Retryable};
use chromiumoxide::{
    browser::HeadlessMode, error::CdpError, handler::browser, Browser, BrowserConfig, Page,
};
use futures::StreamExt;
use sqlx::SqlitePool;
use tokio::time::sleep;

use crate::{config::Config, database::job};

pub async fn init(config: &Config) -> Result<Browser> {
    let mut browser_config = BrowserConfig::builder()
        .user_data_dir(&config.user_data_dir)
        .args(vec![
            &format!("--profile-directory={}", "Default"),
            "--disable-gpu",            // Disable GPU hardware acceleration
            "--disable-extensions",     // Disable extensions
            "--disable-dev-shm-usage",  // Overcome limited resource problems
            "--no-sandbox",             // Bypass OS security model
            "--disable-setuid-sandbox", // Disable the setuid sandbox
            "--disable-infobars",       // Prevent infobars from appearing
            "--disable-notifications",  // Disable web notifications
            "--disable-popup-blocking", // Disable popup blocking
            "--disable-background-timer-throttling", // Disable background timer throttling
            "--disable-backgrounding-occluded-windows", // Disable backgrounding of occluded windows
            "--disable-breakpad",       // Disable the crash reporting
            "--disable-component-extensions-with-background-pages", // Disable component extensions with background pages
            "--disable-features=TranslateUI,BlinkGenPropertyTrees", // Disable specific features
            "--disable-ipc-flooding-protection", // Disable IPC flooding protection
            "--disable-renderer-backgrounding",  // Disable renderer backgrounding
        ])
        .headless_mode(if config.headless {
            HeadlessMode::True
        } else {
            HeadlessMode::False
        });

    if let Some(chrome_path) = &config.chrome_path {
        browser_config = browser_config.chrome_executable(chrome_path);
    }

    let (browser, mut handler) = Browser::launch(browser_config.build().unwrap())
        .await
        .unwrap();

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

pub async fn read(
    chapter_url: &str,
    page: &Page,
    pool: &SqlitePool,
    config: &Config,
) -> Result<()> {
    let read_chapter = || async move { page.goto(chapter_url).await };
    read_chapter
        .retry(ExponentialBuilder::default().with_max_times(config.max_retries))
        .sleep(sleep)
        .when(|error| {
            !matches!(error, CdpError::ChannelSendError(chromiumoxide::error::ChannelError::Send(
                            send_error,
                        )) if send_error.is_disconnected())
        })
        .notify(|err, dur: Duration| {
            tracing::warn!("retrying {:?} after {:?}", err, dur);
        })
        .await
        .with_context(|| format!("Failed to read chapter {}", chapter_url))?;

    sleep(Duration::from_secs(config.wait_for_navigation)).await;

    job::done(chapter_url, pool).await?;

    Ok(())
}
