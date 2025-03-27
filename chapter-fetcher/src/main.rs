use std::{env, sync::LazyLock};

use color_eyre::Result;
use futures::{
    stream::{self},
    StreamExt,
};
use serde::Deserialize;
use serde_json::Value;
use tokio::fs;

static BASE_URL: LazyLock<String> = LazyLock::new(|| env::var("BASE_URL").unwrap());

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
    let raw_data = reqwest::get(format!(
        "{}/api/chapter_list?album={}&page=1&limit=10000&v=1v0",
        BASE_URL.as_str(),
        comic_info.id
    ))
    .await
    .unwrap()
    .text()
    .await
    .unwrap();

    let raw_data: Vec<ChapterRaw> = serde_json::from_str(&raw_data).unwrap();

    raw_data
        .into_iter()
        .map(|chapter_raw| chapter_raw.info)
        .map(|info_raw| serde_json::from_str::<ChapterInfo>(&info_raw).unwrap())
        .map(|chapter_info| get_chapter_url(&chapter_info, comic_info))
        .collect()
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let comics = get_comics().await;
    let chapters: Vec<_> = stream::iter(comics)
        .then(|comic_info| async move { stream::iter(get_chapters(&comic_info).await) })
        .flatten()
        .collect()
        .await;

    fs::write("chapters.txt", chapters.join("\n")).await?;

    Ok(())
}
