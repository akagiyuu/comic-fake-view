use std::time::Duration;

use backon::{ExponentialBuilder, Retryable};
use chromiumoxide::{Browser, BrowserConfig};
use color_eyre::{eyre::Context, Result};
use futures::StreamExt;
use notify_rust::Notification;
use tokio::{fs, time::sleep};

const REMOTE_CHAPTER_LIST: &str =
    "https://raw.githubusercontent.com/akagiyuu/comic-fake-view/refs/heads/main/chapters.txt";
const LOCAL_CHAPTER_LIST: &str = "chapters.txt";

async fn get_chapter_list() -> Result<String> {
    let chapters = if !fs::try_exists(LOCAL_CHAPTER_LIST).await? {
        let chapters = reqwest::get(REMOTE_CHAPTER_LIST).await?.text().await?;
        fs::write(LOCAL_CHAPTER_LIST, &chapters).await?;
        chapters
    } else {
        fs::read_to_string(LOCAL_CHAPTER_LIST).await?
    };

    Ok(chapters)
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let chapters = get_chapter_list().await?;

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
