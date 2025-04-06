use color_eyre::Result;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(flatten)]
    pub browser_config: browser::Config,

    #[serde(flatten)]
    pub automation_config: automation::Config,
}

impl Config {
    pub fn load() -> Result<Self> {
        ::config::Config::builder()
            .add_source(config::Environment::with_prefix("app"))
            .build()
            .and_then(|raw| raw.try_deserialize::<Self>())
            .map_err(color_eyre::eyre::Error::from)
    }
}
