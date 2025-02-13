use std::time::Duration;

use backon::{ExponentialBuilder, Retryable};
use chromiumoxide::browser::{Browser, BrowserConfig};
use color_eyre::{eyre::Context, Result};
use futures::{
    stream::{self},
    StreamExt,
};
use notify_rust::Notification;
use regex::Regex;
use tokio::{fs, time::sleep};

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

// async fn get_chapters(comic_url: String, browser: &Browser) -> impl Stream<Item = String> {
//     let page = browser.new_page(comic_url).await.unwrap();
//     let elements = query_wait(CHAPTER_QUERY, &page).await.unwrap();
//     eprintln!("DEBUGPRINT[110]: main.rs:47: elements={:#?}", elements);
//     page.close().await.unwrap();
//
//     stream::iter(elements).take(1).then(|element| async move {
//         let chapter = element.attribute("href").await.unwrap().unwrap();
//         format!("{BASE_URL}{chapter}")
//     })
// }
//

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

    for chapter_url in chapters.iter() {
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
