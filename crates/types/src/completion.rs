use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::iter::Iterator;
use std::pin::Pin;
use tokio_stream::{Stream, StreamExt};

#[derive(Debug, Clone)]
pub struct Request {
    pub messages: Vec<Message>,
    pub model: Option<String>,
    pub max_tokens: usize,
    pub stop_sequences: Option<Vec<String>>,
    pub stream: bool,
    pub system: Option<String>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct RequestBuilder {
    messages: Vec<Message>,
    model: Option<String>,
    max_tokens: usize,
    stop_sequences: Option<Vec<String>>,
    stream: bool,
    system: Option<String>,
    temperature: Option<f32>,
}

impl Default for RequestBuilder {
    fn default() -> Self {
        Self {
            messages: vec![],
            model: None,
            max_tokens: 100,
            stop_sequences: None,
            stream: false,
            system: None,
            temperature: None,
        }
    }
}

impl RequestBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn model(mut self, model: Option<String>) -> Self {
        self.model = model;
        self
    }

    pub fn messages(mut self, messages: Vec<Message>) -> Self {
        self.messages.extend(messages);
        self
    }

    pub fn max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    pub fn stop_sequences(mut self, stop_sequences: Option<Vec<String>>) -> Self {
        self.stop_sequences = stop_sequences;
        self
    }

    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    pub fn system(mut self, system: Option<String>) -> Self {
        self.system = system;
        self
    }

    pub fn temperature(mut self, temperature: Option<f32>) -> Self {
        self.temperature = temperature;
        self
    }

    pub fn build(self) -> Request {
        Request {
            messages: self.messages,
            model: self.model,
            max_tokens: self.max_tokens,
            stop_sequences: self.stop_sequences,
            stream: self.stream,
            system: self.system,
            temperature: self.temperature,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Usage {
    pub input_tokens: usize,
    pub output_tokens: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Content {
    pub text: String,
    #[serde(rename = "type")]
    pub type_: String,
}

impl From<Content> for Message {
    fn from(content: Content) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.text,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Response {
    pub stop_reason: Option<StopReason>,
    pub stop_sequence: Option<String>,
    pub content: Vec<Content>,
    pub model: String,
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub usage: Usage,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    MaxTokens,
    StopSequence,
    EndTurn,
}

// Equivalent to TypedDict in Python
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl From<String> for Message {
    fn from(content: String) -> Self {
        Self {
            role: "user".to_string(),
            content,
        }
    }
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

impl<'a> MessageDeltaEvent<'a> {
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

impl<'a> UsageEvent<'a> {
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
