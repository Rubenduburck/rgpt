//! Definition of errors used in the library.
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Underlying error from reqwest library after an API call was made
    #[error("http error: {0}")]
    Reqwest(#[from] reqwest::Error),
    /// OpenAI returns error object with details of API call failure
    #[error("{}: {}", .0.r#type, .0.message)]
    Api(ApiError),
    /// Error when a response cannot be deserialized into a Rust type
    #[error("failed to deserialize api response: {0}")]
    JSONDeserialize(serde_json::Error),
    /// Error on SSE streaming
    #[error("stream failed: {0}")]
    Stream(String),
    /// Error from client side validation
    /// or when builder fails to build request before making API call
    #[error("invalid args: {0}")]
    InvalidArgument(String),

    #[error("Serialization error: {0}")]
    JSONSerialize(#[from] serde_json::Error),

    #[error("Caller error: {0}")]
    Caller(#[from] rgpt_caller::error::Error),
}

/// Anthropic API returns error object on failure
#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub message: String,
    pub r#type: String,
    pub param: Option<serde_json::Value>,
    pub code: Option<serde_json::Value>,
}
