use anyhow::Result;
use comic_fake_view_core::config::Config;
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

    let config = Config::load();
    let (_cancellation_sender, cancellation) = watch::channel(false);

    let (progress_notifier, _progress_receiver) = mpsc::unbounded_channel();

    // tokio::spawn(async move {
    //     while let Some((event, data)) = progress_receiver.recv().await {
    //         match event {
    //             _ => {}
    //         }
    //     }
    // });

    comic_fake_view_core::run(progress_notifier, cancellation, &config).await?;

    Ok(())
}
