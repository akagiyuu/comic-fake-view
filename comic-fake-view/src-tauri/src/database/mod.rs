use std::sync::Arc;

use anyhow::Result;
use const_format::formatcp;
use sqlx::SqlitePool;
use tokio::{fs, io::AsyncWriteExt};

pub mod job;

const REMOTE_CHAPTER_LIST: &str =
    "https://raw.githubusercontent.com/akagiyuu/comic-fake-view/refs/heads/main/data.db";
const DATABASE_PATH: &str = "data.db";
const DATABASE_URL: &str = formatcp!("sqlite:{}", DATABASE_PATH);

async fn fetch_remote() -> Result<()> {
    let response = reqwest::get(REMOTE_CHAPTER_LIST).await?;
    let mut out = fs::File::create(DATABASE_PATH).await?;
    let content = response.bytes().await?;
    out.write_all(&content).await?;

    Ok(())
}

async fn create_pool() -> Result<Arc<SqlitePool>> {
    let pool = Arc::new(SqlitePool::connect(DATABASE_URL).await?);
    Ok(pool)
}

pub async fn init() -> Result<Arc<SqlitePool>> {
    if !fs::try_exists(DATABASE_PATH).await.unwrap() {
        fetch_remote().await?;
    }

    let mut pool = create_pool().await?;
    if job::count(pool.as_ref()).await? == 0 {
        fetch_remote().await?;
        pool = create_pool().await?;
    }

    Ok(pool)
}

pub async fn clean(pool: &SqlitePool) -> Result<()> {
    if job::count(pool).await? == 0 {
        fs::remove_file(DATABASE_PATH).await?;
    }

    Ok(())
}
