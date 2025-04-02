use anyhow::Result;
use sqlx::SqlitePool;

pub async fn count(pool: &SqlitePool) -> Result<u64> {
    let count = sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE is_read = false")
        .fetch_one(pool)
        .await?;
    Ok(count)
}

pub async fn is_empty(pool: &SqlitePool) -> Result<bool> {
    count(pool).await.map(|count| count == 0)
}

pub async fn all(pool: &SqlitePool) -> Result<Vec<String>> {
    let jobs = sqlx::query_scalar("SELECT url FROM jobs WHERE is_read = false")
        .fetch_all(pool)
        .await?;
    Ok(jobs)
}

pub async fn done(url: &str, pool: &SqlitePool) -> Result<()> {
    sqlx::query("UPDATE jobs SET is_read = true WHERE url = $1")
        .bind(url)
        .execute(pool)
        .await?;

    Ok(())
}
