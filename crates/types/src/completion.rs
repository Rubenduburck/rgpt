use serde::{Deserialize, Serialize};

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

impl From<Response> for Event {
    fn from(response: Response) -> Self {
        Event::MessageStart {
            message: MessageStartData {
                id: response.id,
                type_: response.type_,
                role: "assistant".to_string(),
                model: response.model,
                content: response.content.to_vec(),
                stop_reason: response.stop_reason,
                stop_sequence: response.stop_sequence,
                usage: response.usage,
            },
        }
    }
}

impl From<Response> for Vec<Event> {
    fn from(response: Response) -> Self {
        vec![Event::from(response), Event::MessageStop]
    }
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

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum Event {
    Ping,
    MessageOpen,
    MessageStart {
        message: MessageStartData,
    },
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },
    ContentBlockDelta {
        index: usize,
        delta: Delta,
    },
    ContentBlockStop {
        index: usize,
    },
    MessageDelta {
        delta: MessageDelta,
    },
    MessageStop,
}

impl Event {
    pub fn text(&self) -> Option<String> {
        match self {
            Event::ContentBlockStart { content_block, .. } => match content_block {
                ContentBlock::Text { text } => Some(text.clone()),
            },
            Event::ContentBlockDelta { delta, .. } => match delta {
                Delta::TextDelta { text } => Some(text.clone()),
            },
            Event::ContentBlockStop { .. } => Some("\n".to_string()),
            _ => None,
        }
    }

    pub fn is_stop(&self) -> bool {
        match self {
            Event::MessageStart { message } => {
                message.stop_reason.is_some() || message.stop_sequence.is_some()
            }
            Event::MessageStop => true,
            Event::ContentBlockStop { .. } => true,
            _ => false,
        }
    }

    pub fn is_complete(&self) -> bool {
        match self {
            Event::MessageStart { message } => message.stop_reason == Some(StopReason::EndTurn),
            Event::MessageStop => true,
            _ => false,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct MessageStartData {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub role: String,
    pub model: String,
    pub content: Vec<Content>,
    pub stop_reason: Option<StopReason>,
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct MessageDelta {
    pub stop_reason: Option<StopReason>,
    pub stop_sequence: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ContentBlock {
    Text { text: String },
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum Delta {
    TextDelta { text: String },
}
