use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use backon::{ExponentialBuilder, Retryable};
use chromiumoxide::{Browser, BrowserConfig};
use futures::{lock::Mutex, stream, StreamExt};
use sqlx::{Connection, SqliteConnection, SqlitePool};
use tauri::{AppHandle, Emitter, Manager};
use tokio::{
    fs::{self, File},
    io,
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
    let body = response.text().await?;
    let mut out = File::create(DATBASE_PATH).await?;
    io::copy(&mut body.as_bytes(), &mut out).await?;

    Ok(())
}

#[tauri::command]
pub async fn run(app_handle: AppHandle) {
    let config = app_handle.state::<Mutex<Config>>();
    let config = config.lock().await.clone();

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
        sender.send(job).unwrap();
    }

    let mut browser_config = BrowserConfig::builder()
        .user_data_dir(&config.user_data_dir)
        .arg(format!("--profile-directory={}", "Default"))
        .with_head();

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
                    .retry(ExponentialBuilder::default())
                    .sleep(sleep)
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
                sqlx::query("UPDATE FROM jobs SET is_open = true WHERE url = $1")
                    .bind(chapter_url)
                    .execute(pool.as_ref())
                    .await.unwrap();
                app_handle.emit("complete", ()).unwrap();
            }

            let _ = page.close().await;
        });
    }

    let job_count: u64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE is_open = false")
        .fetch_one(pool.as_ref())
        .await
        .unwrap();
    if job_count == 0 {
        fs::remove_file(DATBASE_PATH).await.unwrap();
    }

    println!("Finish");
    browser_ref.write().await.close().await.unwrap();
    browser_ref.write().await.wait().await.unwrap();

    app_handle.emit("completed", ()).unwrap();
}
