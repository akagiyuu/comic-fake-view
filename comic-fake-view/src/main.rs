use std::time::Duration;

use backon::{ExponentialBuilder, Retryable};
use chromiumoxide::{Browser, BrowserConfig};
use color_eyre::{eyre::Context, Result};
use futures::StreamExt;
use notify_rust::Notification;
use tokio::{fs, time::sleep};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let chapters = fs::read_to_string("chapters.txt").await?;

    let (mut browser, mut handler) = Browser::launch(
        BrowserConfig::builder()
            .user_data_dir(format!(
                r#"{}\AppData\Local\Google\Chrome\User Data"#,
                std::env::var("USERPROFILE").unwrap()
            ))
            .arg(format!("--profile-directory={}", "Default"))
            .with_head()
            .build()
            .unwrap(),
    )
    .await?;

    let handle = tokio::task::spawn(async move {
        loop {
            let _ = handler.next().await;
        }
    });

    let page = &browser.new_page("about:blank").await?;

    for chapter_url in chapters.lines() {
        let read_chapter = || async { page.goto(chapter_url).await };
        read_chapter
            .retry(ExponentialBuilder::default())
            .sleep(tokio::time::sleep)
            .notify(|err, dur: Duration| {
                println!("retrying {:?} after {:?}", err, dur);
            })
            .await
            .with_context(|| format!("Failed to read chapter {}", chapter_url))?;
        sleep(Duration::from_secs(5)).await;
    }

    browser.close().await.context("Failed to close browser")?;
    browser.wait().await?;

    Notification::new()
        .summary("Tool")
        .body("Tool chạy xong rồi")
        .show()
        .context("Failed to send notification")?;

    handle.await?;

    Ok(())
}
