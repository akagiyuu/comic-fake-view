use color_eyre::Result;
use futures::{
    stream::{self},
    StreamExt,
};
use regex::Regex;
use tokio::fs;

const BASE_URL: &str = "https://cmangag.com";

#[derive(Debug)]
struct ComicInfo {
    id: String,
    url: String,
}

#[derive(Debug)]
struct ChapterInfo {
    id: String,
    num: String,
}

fn query_key(key: &str, data: &str) -> Vec<String> {
    let re = Regex::new(&format!(r#"\\\"{}\\\":\\\"([^"]+)\\\""#, key)).unwrap();
    re.captures_iter(data)
        .map(|cap| cap.get(1).unwrap().as_str().to_string())
        .collect()
}

fn get_chapter_url(chapter_info: &ChapterInfo, comic_info: &ComicInfo) -> String {
    format!(
        "{}/album/{}/chapter-{}-{}",
        BASE_URL, comic_info.url, chapter_info.num, chapter_info.id
    )
}

async fn get_comics() -> Result<Vec<ComicInfo>> {
    let raw_data = reqwest::get(
        "https://cmangag.com/api/home_album_list?file=image&limit=10000&team=5&page=1",
    )
    .await?
    .text()
    .await?;

    Ok(query_key("id", &raw_data)
        .into_iter()
        .zip(query_key("url", &raw_data).into_iter())
        .map(|(id, url)| ComicInfo { id, url })
        .collect())
}

async fn get_chapters(comic_info: &ComicInfo) -> Result<Vec<String>> {
    let raw_data = reqwest::get(format!(
        "https://cmangag.com/api/chapter_list?album={}&page=1&limit=10000&v=1v0",
        comic_info.id
    ))
    .await?
    .text()
    .await?;

    Ok(query_key("id", &raw_data)
        .into_iter()
        .zip(query_key("num", &raw_data).into_iter())
        .map(|(id, num)| ChapterInfo { id, num })
        .map(|chapter_info| get_chapter_url(&chapter_info, comic_info))
        .collect())
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let comics = get_comics().await?;
    let chapters: Vec<_> = stream::iter(comics)
        .then(|comic_info| async move { stream::iter(get_chapters(&comic_info).await.unwrap()) })
        .flatten()
        .collect()
        .await;

    fs::write("chapters.txt", chapters.join("\n")).await?;

    Ok(())
}
