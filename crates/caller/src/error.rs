//! Definition of errors used in the library.
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Underlying error from reqwest library after an API call was made
    #[error("http error: {0}")]
    Reqwest(#[from] reqwest::Error),
    /// OpenAI returns error object with details of API call failure
    #[error("{}: {}", .0.r#type, .0.message)]
    ApiError(ApiError),
    /// Error when a response cannot be deserialized into a Rust type
    #[error("failed to deserialize api response: {0}")]
    JSONDeserialize(serde_json::Error),
    /// Error on SSE streaming
    #[error("stream failed: {0}")]
    StreamError(String),
    /// Error from client side validation
    /// or when builder fails to build request before making API call
    #[error("invalid args: {0}")]
    InvalidArgument(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("reqwest eventsource cannot clone request: {0}")]
    ReqwestEventSource(#[from] reqwest_eventsource::CannotCloneRequestError),
}

/// Anthropic API returns error object on failure
#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub message: String,
    pub r#type: String,
    pub param: Option<serde_json::Value>,
    pub code: Option<serde_json::Value>,
}

/// Wrapper to deserialize the error object nested in "error" JSON key
#[derive(Debug, Deserialize)]
pub(crate) struct WrappedError {
    pub(crate) error: ApiError,
}

pub(crate) fn map_deserialization_error(e: serde_json::Error, _bytes: &[u8]) -> Error {
    Error::JSONDeserialize(e)
}
