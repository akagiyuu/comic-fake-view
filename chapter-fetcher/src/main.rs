use std::{
    env,
    sync::{Arc, LazyLock},
    time::Duration,
};

use backon::{ExponentialBuilder, Retryable};
use color_eyre::{
    eyre::{Context, Error},
    Result,
};
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

#[tracing::instrument]
async fn get_comics() -> Result<Vec<ComicInfo>> {
    let raw_data = reqwest::get(format!(
        "{}/api/home_album_list?file=image&limit=10000&team=5&page=1",
        BASE_URL.as_str()
    ))
    .await?
    .text()
    .await?;
    let raw_data: ComicRawList = serde_json::from_str(&raw_data).context("Empty comic list")?;

    raw_data
        .data
        .into_iter()
        .map(|comic_raw| comic_raw.info)
        .map(|info_raw| serde_json::from_str::<ComicInfo>(&info_raw))
        .collect::<Result<Vec<_>, _>>()
        .map_err(Error::from)
}

#[tracing::instrument]
async fn get_chapters(comic_info: &ComicInfo) -> Result<Vec<String>> {
    let raw_data = reqwest::get(format!(
        "{}/api/chapter_list?album={}&page=1&limit=10000&v=1v0",
        BASE_URL.as_str(),
        comic_info.id
    ))
    .await?
    .text()
    .await?;

    let raw_data: Vec<ChapterRaw> =
        serde_json::from_str(&raw_data).context("Empty chapter list")?;

    Ok(raw_data
        .into_iter()
        .map(|chapter_raw| chapter_raw.info)
        .flat_map(|info_raw| serde_json::from_str::<ChapterInfo>(&info_raw))
        .filter(|info| info.lock.is_none())
        .map(|chapter_info| get_chapter_url(&chapter_info, comic_info))
        .collect())
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

    let comics = get_comics().await?;
    let chapters: Vec<_> = stream::iter(comics)
        .then(|comic_info| async move {
            let wrapper = async || get_chapters(&comic_info).await;
            let chapters = wrapper
                .retry(ExponentialBuilder::default())
                .sleep(sleep)
                .await
                .unwrap_or_default();

            stream::iter(chapters)
        })
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
