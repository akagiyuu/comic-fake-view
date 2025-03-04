use serde::Deserialize;

fn default_user_data_dir() -> String {
    format!(
        r#"{}\AppData\Local\Google\Chrome\User Data"#,
        std::env::var("USERPROFILE").unwrap_or_default()
    )
}

const fn default_wait_for_navigation() -> u64 {
    5000
}

const fn default_max_retries() -> usize {
    3
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub chrome_path: Option<String>,

    #[serde(default = "default_user_data_dir")]
    pub user_data_dir: String,

    pub wait_for_navigation: u64,

    pub max_retries: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            chrome_path: Default::default(),
            user_data_dir: default_user_data_dir(),
            wait_for_navigation: default_wait_for_navigation(),
            max_retries: default_max_retries(),
        }
    }
}
