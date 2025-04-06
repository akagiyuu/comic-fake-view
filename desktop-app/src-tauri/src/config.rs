use color_eyre::Result;
use serde::{Deserialize, Serialize};
use tokio::fs;

const CONFIG_FILE_NAME: &str = "config";

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(flatten)]
    pub automation_config: automation::Config,

    #[serde(flatten)]
    pub browser_config: browser::Config,
}

impl Config {
    pub fn load() -> Result<Self> {
        ::config::Config::builder()
            .add_source(config::File::with_name(CONFIG_FILE_NAME))
            .build()
            .and_then(|raw| raw.try_deserialize::<Self>())
            .map_err(color_eyre::eyre::Error::from)
    }

    pub async fn save(&self) -> Result<()> {
        tracing::info!("new config created: {:?}", self);
        let toml = toml::to_string_pretty(self)?;
        fs::write(format!("{}.toml", CONFIG_FILE_NAME), toml).await?;

        Ok(())
    }
}
