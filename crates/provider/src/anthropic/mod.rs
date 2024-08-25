pub mod provider;
pub mod config;
pub mod error;
pub mod types;
pub mod api_key;

/// A constant to represent the human prompt.
pub const HUMAN_PROMPT: &str = "\n\nHuman:";
/// A constant to represent the assistant prompt.
pub const AI_PROMPT: &str = "\n\nAssistant:";

/// Default model to use.
pub const DEFAULT_MODEL: &str = "claude-instant-1.2";
pub const DEFAULT_MAX_TOKENS: usize = 100;
/// Default v1 API base url.
pub const API_BASE: &str = "https://api.anthropic.com";
/// Auth header key.
const AUTHORIZATION_HEADER_KEY: &str = "x-api-key";
/// Client id header key.
const CLIENT_ID_HEADER_KEY: &str = "Client";
/// API version header key.
/// Ref: https://docs.anthropic.com/claude/reference/versioning
const API_VERSION_HEADER_KEY: &str = "anthropic-version";

lazy_static::lazy_static! {
    /// A value to represent the client id of this SDK.
    pub static ref CLIENT_ID: String = client_id();
}

/// Ref: https://docs.anthropic.com/claude/reference/versioning
const API_VERSION: &str = "2023-06-01";

/// Get the client id.
pub fn client_id() -> String {
    // Get the Rust version used to build SDK at compile time.
    let rust_version = match rustc_version::version() {
        Ok(v) => v.to_string(),
        Err(_) => "unknown".to_string(),
    };
    let crate_name = env!("CARGO_PKG_NAME");
    let crate_version = env!("CARGO_PKG_VERSION");
    format!("rustv{rust_version}/{crate_name}/{crate_version}")
}
