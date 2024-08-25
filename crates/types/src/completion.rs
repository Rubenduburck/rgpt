use std::collections::HashMap;
use std::iter::Iterator;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Deserialize, Clone, PartialEq, Serialize)]
pub struct CompleteResponse {
    pub completion: String,
    pub stop_reason: Option<StopReason>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    MaxTokens,
    StopSequence,
}



// Equivalent to TypedDict in Python
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

// Equivalent to TypedDict with total=False
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ModelOverrides {
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
}

// Equivalent to TypedDict
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Pricing {
    pub prompt: f32,
    pub response: f32,
}

// Equivalent to dataclass
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageDeltaEvent<'a> {
    text: String,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    r#type: &'a str,
}

impl <'a>MessageDeltaEvent<'a> {
    fn new(text: String) -> Self {
        Self {
            text,
            r#type: "message_delta",
        }
    }
}

// Equivalent to dataclass
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UsageEvent<'a> {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
    cost: f32,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    r#type: &'a str,
}

impl <'a>UsageEvent<'a> {
    fn new(prompt_tokens: i32, completion_tokens: i32, total_tokens: i32, cost: f32) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens,
            cost,
            r#type: "usage",
        }
    }

    fn with_pricing(
        prompt_tokens: i32,
        completion_tokens: i32,
        total_tokens: i32,
        pricing: &Pricing,
    ) -> Self {
        Self::new(
            prompt_tokens,
            completion_tokens,
            total_tokens,
            prompt_tokens as f32 * pricing.prompt + completion_tokens as f32 * pricing.response,
        )
    }
}

// Equivalent to Union
#[derive(Debug, Clone)]
pub enum CompletionEvent<'a> {
    MessageDelta(MessageDeltaEvent<'a>),
    Usage(UsageEvent<'a>),
}

// Equivalent to abstract base class
pub trait CompletionProvider {
    fn complete(
        &self,
        messages: &[Message],
        args: &HashMap<String, String>,
        stream: bool,
    ) -> Box<dyn Iterator<Item = CompletionEvent>>;
}

// Custom error types
#[derive(Debug, Deserialize, Serialize)]
pub struct CompletionError;

#[derive(Debug, Deserialize, Serialize)]
pub struct BadRequestError;

impl std::error::Error for CompletionError {}
impl std::fmt::Display for CompletionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Completion error")
    }
}

impl std::error::Error for BadRequestError {}
impl std::fmt::Display for BadRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bad request error")
    }
}
