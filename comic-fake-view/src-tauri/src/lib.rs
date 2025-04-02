use std::fs::File;

use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub mod command;
pub mod config;
pub mod database;
pub mod browser;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_file = File::create("tracing.log").unwrap();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(log_file),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .pretty()
                .with_timer(fmt::time::ChronoLocal::rfc_3339()),
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
            command::run
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
