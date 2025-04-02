use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use backon::{ExponentialBuilder, Retryable};
use chromiumoxide::{browser::HeadlessMode, error::CdpError, Browser, BrowserConfig};
use futures::{channel::mpsc::SendError, lock::Mutex, StreamExt};
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter, Manager};
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
    sync::RwLock,
    task::JoinSet,
    time::sleep,
};

use crate::config::Config;

const REMOTE_CHAPTER_LIST: &str =
    "https://raw.githubusercontent.com/akagiyuu/comic-fake-view/refs/heads/main/data.db";
const DATBASE_PATH: &str = "data.db";

async fn get_chapter_list() -> Result<()> {
    let response = reqwest::get(REMOTE_CHAPTER_LIST).await?;
    let mut out = File::create(DATBASE_PATH).await?;
    let content = response.bytes().await?;
    out.write_all(&content).await?;

    Ok(())
}

#[tauri::command]
pub async fn run(app_handle: AppHandle) {
    let config = Config::load();

    if !fs::try_exists(DATBASE_PATH).await.unwrap() {
        get_chapter_list().await.unwrap();
    }
    let pool = Arc::new(
        SqlitePool::connect(&format!("sqlite:{}", DATBASE_PATH))
            .await
            .unwrap(),
    );

    let (sender, receiver) = flume::unbounded::<String>();
    let jobs: Vec<String> = sqlx::query_scalar("SELECT url FROM jobs WHERE is_read = false")
        .fetch_all(pool.as_ref())
        .await
        .unwrap();
    app_handle.emit("total_jobs", jobs.len()).unwrap();

    for job in jobs {
        sender.send_async(job).await.unwrap();
    }

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

    tauri::async_runtime::spawn(async move {
        loop {
            let _ = handler.next().await;
        }
    });

    let browser_ref = Arc::new(RwLock::new(browser));
    let config = Arc::new(config);
    let mut join_set = JoinSet::new();
    for _ in 0..config.tab_count {
        let app_handle = app_handle.clone();
        let receiver = receiver.clone();

        let browser_ref = browser_ref.clone();
        let config = config.clone();
        let pool = pool.clone();
        join_set.spawn(async move {
            let page = browser_ref
                .read()
                .await
                .new_page("about:blank")
                .await
                .unwrap();
            let page_ref = &page;

            while let Ok(chapter_url) = receiver.recv_timeout(Duration::from_secs(10)) {
                let chapter_url = &chapter_url;
                let read_chapter = || async move { page_ref.goto(chapter_url).await };
                if let Err(error) = read_chapter
                    .retry(ExponentialBuilder::default().with_max_times(config.max_retries))
                    .sleep(sleep)
                    .when(|error| !matches!(error, CdpError::ChannelSendError(chromiumoxide::error::ChannelError::Send(
                            send_error,
                        )) if send_error.is_disconnected()))
                    .notify(|err, dur: Duration| {
                        println!("retrying {:?} after {:?}", err, dur);
                    })
                    .await
                    .with_context(|| format!("Failed to read chapter {}", chapter_url))
                {
                    println!("{}", error);
                    app_handle.emit("error", error.to_string()).unwrap();
                    break;
                }
                sleep(Duration::from_secs(config.wait_for_navigation)).await;
                sqlx::query("UPDATE jobs SET is_read = true WHERE url = $1")
                    .bind(chapter_url)
                    .execute(pool.as_ref())
                    .await
                    .unwrap();
                app_handle.emit("complete", ()).unwrap();
            }

            let _ = page.close().await;
        });
    }

    while join_set.join_next().await.is_some() {}

    let job_count: u64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE is_read = false")
        .fetch_one(pool.as_ref())
        .await
        .unwrap();
    if job_count == 0 {
        fs::remove_file(DATBASE_PATH).await.unwrap();
    }

    tokio::spawn(async move {
        browser_ref.write().await.close().await.unwrap();
    });
    println!("Finish");

    app_handle.emit("completed", ()).unwrap();
}
