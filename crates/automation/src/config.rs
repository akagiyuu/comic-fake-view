use serde::{Deserialize, Serialize};

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
pub struct Config {
    #[serde(flatten)]
    pub browser_config: browser::config::Config,

    pub wait_for_navigation: u64,

    pub max_retries: usize,

    pub tab_count: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            browser_config: Default::default(),
            wait_for_navigation: default_wait_for_navigation(),
            max_retries: default_max_retries(),
            tab_count: default_tab_count(),
        }
    }
}
