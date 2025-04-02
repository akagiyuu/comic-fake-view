use std::{
    env,
    sync::{Arc, LazyLock},
    time::Duration,
};

use backon::{ExponentialBuilder, Retryable};
use color_eyre::{eyre::Error, Result};
use futures::{
    stream::{self},
    StreamExt,
};
use serde::Deserialize;
use serde_json::Value;
use sqlx::{sqlite::SqlitePoolOptions, Executor};
use tokio::{fs, time::sleep};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

static BASE_URL: LazyLock<String> = LazyLock::new(|| env::var("BASE_URL").unwrap());
const DATABASE_PATH: &str = "data.db";
const SCHEMA: &str = include_str!("../../schema.sql");

#[derive(Debug, Deserialize)]
struct ComicInfo {
    id: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct ComicRaw {
    info: String,
}

#[derive(Debug, Deserialize)]
struct ComicRawList {
    data: Vec<ComicRaw>,
}

#[derive(Debug, Deserialize)]
struct ChapterRaw {
    info: String,
}

#[derive(Debug, Deserialize)]
struct ChapterInfo {
    id: Value,
    num: String,
    lock: Option<Value>,
}

fn get_chapter_url(chapter_info: &ChapterInfo, comic_info: &ComicInfo) -> String {
    let chapter_id = chapter_info.id.to_string();

    format!(
        "{}/album/{}/chapter-{}-{}",
        BASE_URL.as_str(),
        comic_info.url,
        chapter_info.num,
        &chapter_id[1..chapter_id.len() - 1]
    )
}

async fn get_comics() -> Vec<ComicInfo> {
    let raw_data = reqwest::get(format!(
        "{}/api/home_album_list?file=image&limit=10000&team=5&page=1",
        BASE_URL.as_str()
    ))
    .await
    .unwrap()
    .text()
    .await
    .unwrap();
    let raw_data: ComicRawList = serde_json::from_str(&raw_data).unwrap();

    raw_data
        .data
        .into_iter()
        .map(|comic_raw| comic_raw.info)
        .map(|info_raw| serde_json::from_str::<ComicInfo>(&info_raw).unwrap())
        .collect()
}

async fn get_chapters(comic_info: &ComicInfo) -> Vec<String> {
    let get_chapter = || async move {
        let raw_data = reqwest::get(format!(
            "{}/api/chapter_list?album={}&page=1&limit=10000&v=1v0",
            BASE_URL.as_str(),
            comic_info.id
        ))
        .await
        .map_err(Error::from)?
        .text()
        .await
        .map_err(Error::from)?;

        if raw_data.is_empty() {
            Err(color_eyre::eyre::anyhow!("Empty chapter"))
        } else {
            Ok(raw_data)
        }
    };

    let raw_data = get_chapter
        .retry(ExponentialBuilder::default())
        .sleep(sleep)
        .await
        .unwrap_or_default();

    let raw_data: Vec<ChapterRaw> = match serde_json::from_str(&raw_data) {
        Ok(v) => v,
        Err(error) => {
            tracing::error!("{:?}", comic_info);
            tracing::error!("{}", error);
            vec![]
        }
    };

    raw_data
        .into_iter()
        .map(|chapter_raw| chapter_raw.info)
        .map(|info_raw| serde_json::from_str::<ChapterInfo>(&info_raw).unwrap())
        .filter(|info| info.lock.is_none())
        .map(|chapter_info| get_chapter_url(&chapter_info, comic_info))
        .collect()
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .pretty()
                .with_timer(fmt::time::ChronoLocal::rfc_3339()),
        )
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let comics = get_comics().await;
    let chapters: Vec<_> = stream::iter(comics)
        .then(|comic_info| async move { stream::iter(get_chapters(&comic_info).await) })
        .flatten()
        .collect()
        .await;

    tracing::info!("Number of jobs: {}", chapters.len());

    fs::File::create(DATABASE_PATH).await?;

    let pool = Arc::new(
        SqlitePoolOptions::new()
            .acquire_slow_threshold(Duration::MAX)
            .connect(&format!("sqlite:{}", DATABASE_PATH))
            .await?,
    );
    pool.execute(SCHEMA).await?;

    for url in chapters {
        if let Err(error) = sqlx::query("INSERT INTO jobs(url) VALUES($1)")
            .bind(url)
            .execute(pool.as_ref())
            .await
        {
            tracing::error!("{}", error);
        }
    }

    Ok(())
}
