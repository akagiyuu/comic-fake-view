use anyhow::Result;
use sqlx::SqlitePool;

pub async fn count(pool: &SqlitePool) -> Result<u64> {
    let count = sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE is_read = false")
        .fetch_one(pool)
        .await?;
    Ok(count)
}

pub async fn all(pool: &SqlitePool) -> Result<flume::Receiver<String>> {
    let jobs = sqlx::query_scalar("SELECT url FROM jobs WHERE is_read = false LIMIT 10")
        .fetch_all(pool)
        .await?;

    let (sender, receiver) = flume::unbounded::<String>();

    for job in jobs {
        sender.send_async(job).await?;
    }

    Ok(receiver)
}

pub async fn done(url: &str, pool: &SqlitePool) -> Result<()> {
    sqlx::query("UPDATE jobs SET is_read = true WHERE url = $1")
        .bind(url)
        .execute(pool)
        .await?;

    Ok(())
}
