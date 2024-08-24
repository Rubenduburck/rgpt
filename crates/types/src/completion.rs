use std::collections::HashMap;
use std::iter::Iterator;

// Equivalent to TypedDict in Python
#[derive(Debug, Clone)]
struct Message {
    role: String,
    content: String,
}

// Equivalent to TypedDict with total=False
#[derive(Debug, Clone, Default)]
struct ModelOverrides {
    model: Option<String>,
    temperature: Option<f32>,
    top_p: Option<f32>,
}

// Equivalent to TypedDict
#[derive(Debug, Clone)]
struct Pricing {
    prompt: f32,
    response: f32,
}

// Equivalent to dataclass
#[derive(Debug, Clone)]
struct MessageDeltaEvent {
    text: String,
    #[allow(dead_code)]
    r#type: &'static str,
}

impl MessageDeltaEvent {
    fn new(text: String) -> Self {
        Self {
            text,
            r#type: "message_delta",
        }
    }
}

// Equivalent to dataclass
#[derive(Debug, Clone)]
struct UsageEvent {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
    cost: f32,
    #[allow(dead_code)]
    r#type: &'static str,
}

impl UsageEvent {
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
enum CompletionEvent {
    MessageDelta(MessageDeltaEvent),
    Usage(UsageEvent),
}

// Equivalent to abstract base class
trait CompletionProvider {
    fn complete(
        &self,
        messages: &[Message],
        args: &HashMap<String, String>,
        stream: bool,
    ) -> Box<dyn Iterator<Item = CompletionEvent>>;
}

// Custom error types
#[derive(Debug)]
struct CompletionError;

#[derive(Debug)]
struct BadRequestError;

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
