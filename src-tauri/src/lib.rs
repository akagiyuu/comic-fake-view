use std::fs::File;

use tauri::Manager;
use tokio::sync::{watch, Mutex};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    filter, fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
};

pub mod browser;
pub mod command;
pub mod config;
pub mod database;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_file = File::create("tracing.log").unwrap();
    let filter = filter::filter_fn(|metadata| !metadata.target().contains("chromiumoxide"));

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(log_file)
                .with_filter(filter.clone()),
        )
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

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            command::set_config,
            command::get_config,
            command::stop,
            command::run
        ])
        .setup(|app_handle| {
            let (set_is_stopped, is_stopped) = watch::channel(false);
            app_handle.manage(is_stopped);
            app_handle.manage(Mutex::new(set_is_stopped));

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
