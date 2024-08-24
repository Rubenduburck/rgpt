pub mod provider;
pub mod config;
pub mod error;
pub mod types;

/// A constant to represent the human prompt.
pub const HUMAN_PROMPT: &str = "\n\nHuman:";
/// A constant to represent the assistant prompt.
pub const AI_PROMPT: &str = "\n\nAssistant:";

/// Default model to use.
pub const DEFAULT_MODEL: &str = "claude-v1";
/// Default v1 API base url.
pub const DEFAULT_API_BASE: &str = "https://api.anthropic.com";
/// Auth header key.
const AUTHORIZATION_HEADER_KEY: &str = "x-api-key";
/// Client id header key.
const CLIENT_ID_HEADER_KEY: &str = "Client";
/// API version header key.
/// Ref: https://docs.anthropic.com/claude/reference/versioning
const API_VERSION_HEADER_KEY: &str = "anthropic-version";

/// Ref: https://docs.anthropic.com/claude/reference/versioning
const API_VERSION: &str = "2023-06-01";
