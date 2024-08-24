//! Module for types used in the API.
use std::pin::Pin;

use serde::{Deserialize, Serialize};
use tokio_stream::Stream;

use crate::anthropic::error::Error;
use crate::anthropic::DEFAULT_MODEL;

#[derive(Clone, Serialize, Debug, PartialEq)]
pub struct CompleteRequest {
    /// The prompt to complete.
    pub prompt: String,
    /// The model to use.
    pub model: String,
    /// The number of tokens to sample.
    pub max_tokens_to_sample: usize,
    /// The stop sequences to use.
    pub stop_sequences: Option<Vec<String>>,
    /// Whether to incrementally stream the response.
    pub stream: bool,
}

impl Default for CompleteRequest {
    fn default() -> Self {
        Self {
            prompt: "".to_string(),
            model: DEFAULT_MODEL.to_string(),
            max_tokens_to_sample: 100,
            stop_sequences: None,
            stream: false,
        }
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Serialize)]
pub struct CompleteResponse {
    pub completion: String,
    pub stop_reason: Option<StopReason>,
}

/// Parsed server side events stream until a [StopReason::StopSequence] is received from server.
pub type CompleteResponseStream = Pin<Box<dyn Stream<Item = Result<CompleteResponse, rgpt_caller::error::Error>> + Send>>;

#[derive(Debug, Deserialize, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    MaxTokens,
    StopSequence,
}
