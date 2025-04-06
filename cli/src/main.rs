pub mod config;

use std::sync::Arc;

use color_eyre::Result;
use config::Config;
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
    let (_cancellation_sender, cancellation) = watch::channel(false);

    let (progress_notifier, mut progress_receiver) =
        mpsc::unbounded_channel::<(&'static str, String)>();
    let automation_handler = tokio::spawn(async move {
        automation::run(
            progress_notifier,
            cancellation,
            Arc::new(config.automation_config),
            &config.browser_config,
        )
        .await
    });

    let (_, job_count) = progress_receiver.recv().await.unwrap();

    let progress_handler = tokio::spawn(async move {
        let mut count = 0;
        while let Some((event, _)) = progress_receiver.recv().await {
            if let "complete" = event {
                count += 1;
                tracing::info!("Progress: {}/{}", count, job_count);
            }
        }
    });

    let _ = tokio::join!(automation_handler, progress_handler);

    Ok(())
}
