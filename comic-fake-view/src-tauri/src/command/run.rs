use std::time::Duration;

use anyhow::{Context, Result};
use backon::{ExponentialBuilder, Retryable};
use chromiumoxide::{Browser, BrowserConfig};
use futures::{lock::Mutex, stream, StreamExt};
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::sleep;

use crate::config::Config;

const REMOTE_CHAPTER_LIST: &str =
    "https://raw.githubusercontent.com/akagiyuu/comic-fake-view/refs/heads/main/chapters.txt";

async fn get_chapter_list() -> Result<String> {
    let chapters = reqwest::get(REMOTE_CHAPTER_LIST)
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    Ok(chapters)
}

#[tauri::command]
pub async fn run(app_handle: AppHandle) {
    let config = app_handle.state::<Mutex<Config>>();
    let config = config.lock().await;

    let channel = app_handle.state::<(flume::Sender<String>, flume::Receiver<String>)>();
    if channel.0.is_empty() {
        let chapters = get_chapter_list().await.unwrap();
        stream::iter(chapters.lines())
            .for_each_concurrent(None, |chapter| async {
                channel.0.send_async(chapter.to_string()).await.unwrap();
            })
            .await;
    }

    app_handle.emit("total_jobs", channel.0.len()).unwrap();

    let mut browser_config = BrowserConfig::builder()
        .user_data_dir(&config.user_data_dir)
        .arg(format!("--profile-directory={}", "Default"))
        .with_head();

    if let Some(chrome_path) = &config.chrome_path {
        browser_config = browser_config.chrome_executable(chrome_path);
    }

    let (mut browser, mut handler) = Browser::launch(browser_config.build().unwrap())
        .await
        .unwrap();

    tauri::async_runtime::spawn(async move {
        loop {
            let _ = handler.next().await;
        }
    });

    let browser_ref = &browser;
    let config = &config;

    stream::iter(0..20)
        .for_each_concurrent(config.tab_count, |_| {
            let app_handle = app_handle.clone();
            let receiver = channel.1.clone();

            async move {
                let page = browser_ref.new_page("about:blank").await.unwrap();
                let page_ref = &page;

                while let Ok(chapter_url) = receiver.recv_async().await {
                    let chapter_url = &chapter_url;
                    let read_chapter = || async move { page_ref.goto(chapter_url).await };
                    if read_chapter
                        .retry(ExponentialBuilder::default())
                        .sleep(sleep)
                        .notify(|err, dur: Duration| {
                            println!("retrying {:?} after {:?}", err, dur);
                        })
                        .await
                        .with_context(|| format!("Failed to read chapter {}", chapter_url))
                        .is_err()
                    {
                        break;
                    }
                    sleep(Duration::from_secs(config.wait_for_navigation)).await;
                    app_handle.emit("complete", ()).unwrap();
                }

                let _ = page.close().await;
            }
        })
        .await;

    browser
        .close()
        .await
        .context("Failed to close browser")
        .unwrap();
    browser.wait().await.unwrap();

    app_handle.emit("completed", ()).unwrap();
}
