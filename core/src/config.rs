use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::fs;

const CONFIG_FILE_NAME: &str = "config";

#[cfg(target_os = "windows")]
fn default_user_data_dir() -> String {
    format!(
        r#"{}\AppData\Local\Google\Chrome\User Data"#,
        std::env::var("USERPROFILE").unwrap_or_default()
    )
}

#[cfg(not(target_os = "windows"))]
fn default_user_data_dir() -> String {
    "~/.chromium".to_string()
}

const fn default_wait_for_navigation() -> u64 {
    1
}

const fn default_max_retries() -> usize {
    10
}

const fn default_tab_count() -> usize {
    5
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub chrome_path: Option<String>,

    #[serde(default = "default_user_data_dir")]
    pub user_data_dir: String,

    pub headless: bool,

    pub wait_for_navigation: u64,

    pub max_retries: usize,

    pub tab_count: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            chrome_path: Default::default(),
            user_data_dir: default_user_data_dir(),
            headless: false,
            wait_for_navigation: default_wait_for_navigation(),
            max_retries: default_max_retries(),
            tab_count: default_tab_count(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        ::config::Config::builder()
            .add_source(config::File::with_name(CONFIG_FILE_NAME))
            .add_source(
                config::Environment::with_prefix("APP")
                    .try_parsing(true)
                    .separator("_")
                    .list_separator(" "),
            )
            .build()
            .and_then(|raw| raw.try_deserialize::<Self>())
            .unwrap_or_default()
    }

    pub async fn save(&self) -> Result<()> {
        tracing::info!("new config created: {:?}", self);
        let toml = toml::to_string_pretty(self)?;
        fs::write(format!("{}.toml", CONFIG_FILE_NAME), toml).await?;

        Ok(())
    }
}
