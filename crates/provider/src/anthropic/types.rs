//! Module for types used in the API.
use std::pin::Pin;

use serde::{Deserialize, Serialize};
use tokio_stream::Stream;

use crate::anthropic::DEFAULT_MODEL;

use super::DEFAULT_MAX_TOKENS;

#[derive(Debug, Deserialize, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    MaxTokens,
    StopSequence,
}

// Completion API
#[derive(Clone, Serialize, Debug, PartialEq)]
pub struct CompleteRequest {
    pub prompt: String,
    pub model: String,
    pub max_tokens_to_sample: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<usize>,
}

impl Default for CompleteRequest {
    fn default() -> Self {
        Self {
            prompt: "".to_string(),
            model: DEFAULT_MODEL.to_string(),
            max_tokens_to_sample: DEFAULT_MAX_TOKENS,
            stop_sequences: None,
            stream: false,
            temperature: None,
            top_p: None,
            top_k: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Serialize)]
pub struct CompleteResponse {
    pub completion: String,
    pub stop_reason: Option<StopReason>,
    pub model: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub id: String,
}

pub type CompleteResponseStream =
    Pin<Box<dyn Stream<Item = Result<CompleteResponse, rgpt_caller::error::Error>> + Send>>;


// Messages API
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessagesRequest {
    pub messages: Vec<Message>,
    pub model: String,
    pub max_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

impl Default for MessagesRequest {
    fn default() -> Self {
        Self {
            messages: vec![],
            model: DEFAULT_MODEL.to_string(),
            max_tokens: DEFAULT_MAX_TOKENS,
            stop_sequences: None,
            stream: false,
            system: None,
            temperature: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Content {
    pub text: String,
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Usage {
    input_tokens: usize,
    output_tokens: usize,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct MessagesResponse {
    pub stop_reason: Option<StopReason>,
    pub stop_sequence: Option<String>,
    pub content: Vec<Content>,
    pub model: String,
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub usage: Usage,
}

pub type MessagesResponseStream =
    Pin<Box<dyn Stream<Item = Result<MessagesResponse, rgpt_caller::error::Error>> + Send>>;
