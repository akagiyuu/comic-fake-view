use std::time::Duration;

use anyhow::{Context, Result};
use backon::{ExponentialBuilder, Retryable};
use chromiumoxide::{Browser, BrowserConfig};
use futures::{lock::Mutex, StreamExt};
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::sleep;

use crate::config::Config;

const REMOTE_CHAPTER_LIST: &str =
    "https://raw.githubusercontent.com/akagiyuu/comic-fake-view/refs/heads/main/chapters.txt";

async fn get_chapter_list() -> Result<String> {
    let chapters = reqwest::get(REMOTE_CHAPTER_LIST).await.unwrap().text().await.unwrap();

    Ok(chapters)
}

#[tauri::command]
pub async fn run(app_handle: AppHandle) {
    let config = app_handle.state::<Mutex<Config>>();
    let config = config.lock().await;

    let chapters = get_chapter_list().await.unwrap();

    app_handle.emit("total_jobs", chapters.lines().count()).unwrap();

    let mut browser_config = BrowserConfig::builder()
        .user_data_dir(&config.user_data_dir)
        .arg(format!("--profile-directory={}", "Default"))
        .with_head();

    if let Some(chrome_path) = &config.chrome_path {
        browser_config = browser_config.chrome_executable(chrome_path);
    }

    let (mut browser, mut handler) = Browser::launch(browser_config.build().unwrap()).await.unwrap();

    tauri::async_runtime::spawn(async move {
        loop {
            let _ = handler.next().await;
        }
    });

    let page = &browser.new_page("about:blank").await.unwrap();

    for chapter_url in chapters.lines() {
        let read_chapter = || async { page.goto(chapter_url).await };
        read_chapter
            .retry(ExponentialBuilder::default())
            .sleep(sleep)
            .notify(|err, dur: Duration| {
                println!("retrying {:?} after {:?}", err, dur);
            })
            .await
            .with_context(|| format!("Failed to read chapter {}", chapter_url)).unwrap();
        app_handle.emit("complete", ()).unwrap();
        sleep(Duration::from_secs(config.wait_for_navigation)).await;
    }

    browser.close().await.context("Failed to close browser").unwrap();
    browser.wait().await.unwrap();

    app_handle.emit("completed", ()).unwrap();
}
