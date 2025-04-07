pub mod config;

use std::sync::Arc;

use automation::Message;
use color_eyre::Result;
use config::Config;
use futures::StreamExt;
use tokio::sync::{mpsc, watch};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    EnvFilter, Layer, filter, fmt, layer::SubscriberExt, util::SubscriberInitExt,
};

#[tokio::main]
async fn main() -> Result<()> {
    let filter = filter::filter_fn(|metadata| !metadata.target().contains("chromiumoxide"));

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .pretty()
                .with_timer(fmt::time::ChronoLocal::rfc_3339())
                .with_filter(filter),
        )
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let config = Config::load()?;

    let task = automation::run(
        None,
        Arc::new(config.automation_config),
        &config.browser_config,
    )
    .await;

    tokio::pin!(task);

    let mut count = 0;
    while let Some(progress) = task.next().await {
        match progress? {
            Message::CompleteJob => {
                count += 1;
                tracing::info!("Progress: {}/{}", count, count);
            }
            Message::JobCount(c) => count = c,
        }
    }

    Ok(())
}
