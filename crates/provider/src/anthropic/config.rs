use serde::Deserialize;

/// Configuration for the application.
#[derive(Debug, Deserialize)]
pub struct AnthropicConfig {
    pub api_key: String,
    pub api_base: Option<String>,
    pub default_model: Option<String>,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: "".to_string(),
            api_base: None,
            default_model: None,
        }
    }
}
