use chromiumoxide::{BrowserConfig, browser::HeadlessMode};
use serde::{Deserialize, Serialize};

fn default_user_data_dir() -> String {
    format!(
        r#"{}\AppData\Local\Google\Chrome\User Data"#,
        std::env::var("USERPROFILE").unwrap_or_default()
    )
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub chrome_path: Option<String>,

    #[serde(default = "default_user_data_dir")]
    pub user_data_dir: String,

    pub headless: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            chrome_path: Default::default(),
            user_data_dir: default_user_data_dir(),
            headless: false,
        }
    }
}

const BROWSER_ARGS: [&str; 16] = [
    "--profile-directory=Default",
    "--disable-gpu",
    "--disable-extensions",
    "--disable-dev-shm-usage",
    "--no-sandbox",
    "--disable-setuid-sandbox",
    "--disable-infobars",
    "--disable-notifications",
    "--disable-popup-blocking",
    "--disable-background-timer-throttling",
    "--disable-backgrounding-occluded-windows",
    "--disable-breakpad",
    "--disable-component-extensions-with-background-pages",
    "--disable-features=TranslateUI,BlinkGenPropertyTrees",
    "--disable-ipc-flooding-protection",
    "--disable-renderer-backgrounding",
];

impl TryFrom<&Config> for BrowserConfig {
    type Error = color_eyre::eyre::Error;

    fn try_from(config: &Config) -> std::result::Result<Self, Self::Error> {
        let mut browser_config = BrowserConfig::builder()
            .user_data_dir(&config.user_data_dir)
            .args(BROWSER_ARGS)
            .headless_mode(if config.headless {
                HeadlessMode::True
            } else {
                HeadlessMode::False
            });

        if let Some(chrome_path) = &config.chrome_path {
            browser_config = browser_config.chrome_executable(chrome_path);
        }

        browser_config
            .build()
            .map_err(|error| color_eyre::eyre::anyhow!(error))
    }
}
