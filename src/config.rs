use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub brave_api_key: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
            brave_api_key: std::env::var("BRAVE_API_KEY").ok(),
        }
    }
}
