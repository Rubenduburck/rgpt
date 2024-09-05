use serde::{Deserialize, Serialize};

use crate::message::Message;

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

impl Request {
    pub fn builder() -> RequestBuilder {
        RequestBuilder::new()
    }
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
            max_tokens: 4096,
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

    pub fn model(mut self, model: String) -> Self {
        self.model = Some(model);
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

    pub fn system(mut self, system: String) -> Self {
        self.system = Some(system);
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

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum Content {
    Text{
        text: String,
    },
    Other,
}

impl Content {
    pub fn text(&self) -> Option<String> {
        match self {
            Content::Text{text} => Some(text.clone()),
            _ => None,
        }
    }

    pub fn bytes(&self) -> Vec<u8> {
        match self {
            Content::Text{text} => text.as_bytes().to_vec(),
            _ => vec![],
        }
    }
}

impl From<Content> for Message {
    fn from(content: Content) -> Self {
        match content {
            Content::Text{text} => Message::from(text),
            Content::Other => Message::from("".to_string()),
        }
    }
}

//{\"id\":\"msg_01UZHWJDoDcy78R6YtbPqpHN\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-3-5-sonnet-20240620\",\"content\":[{\"type\":\"text\",\"text\":\"The bartender nods and asks, \\\"Any particular type of beer you're in the mood for? We've got lagers, ales, stouts, and some local craft beers on tap.\\\"\"}],\"stop_reason\":\"end_turn\",\"stop_sequence\":null,\"usage\"
//:{\"input_tokens\":45,\"output_tokens\":44}}
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Response {
    pub stop_reason: Option<StopReason>,
    pub stop_sequence: Option<String>,
    pub content: Vec<Content>,
    pub model: String,
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub role: String,
    pub usage: Usage,
}

impl From<Response> for TextEvent {
    fn from(response: Response) -> Self {
        TextEvent::MessageStart {
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

impl From<Response> for Vec<TextEvent> {
    fn from(response: Response) -> Self {
        vec![TextEvent::from(response), TextEvent::MessageStop]
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    MaxTokens,
    StopSequence,
    EndTurn,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum TextEvent {
    Null,
    MessageStart {
        message: MessageStartData,
    },
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },
    ContentBlockDelta {
        index: usize,
        delta: ContentDelta,
    },
    ContentBlockStop {
        index: usize,
    },
    MessageDelta {
        delta: MessageDelta,
    },
    MessageStop,
}

impl TextEvent {
    pub fn text(&self) -> Option<String> {
        match self {
            TextEvent::ContentBlockStart { content_block, .. } => content_block.text(),
            TextEvent::ContentBlockDelta { delta, .. } => delta.text(),
            TextEvent::ContentBlockStop { .. } => Some("\n".to_string()),
            _ => None,
        }
    }

    pub fn is_stop(&self) -> bool {
        match self {
            TextEvent::MessageStart { message } => {
                message.stop_reason.is_some() || message.stop_sequence.is_some()
            }
            TextEvent::MessageStop => true,
            TextEvent::ContentBlockStop { .. } => true,
            _ => false,
        }
    }

    pub fn is_complete(&self) -> bool {
        match self {
            TextEvent::MessageStart { message } => message.stop_reason == Some(StopReason::EndTurn),
            TextEvent::MessageStop => true,
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
    Other,
}

impl ContentBlock {
    pub fn update(&mut self, delta: &ContentDelta) {
        match (self, delta) {
            (ContentBlock::Text { text }, ContentDelta::TextDelta { text: ref delta }) => {
                text.push_str(delta);
            }
            _ => {
                tracing::error!("Invalid delta update");
            }
        }
    }

    pub fn text(&self) -> Option<String> {
        match self {
            ContentBlock::Text { text } => Some(text.clone()),
            _ => None,
        }
    }

    pub fn bytes(&self) -> Vec<u8> {
        match self {
            ContentBlock::Text { text } => text.as_bytes().to_vec(),
            _ => vec![],
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ContentDelta {
    TextDelta { text: String },
    Other,
}

impl ContentDelta {
    pub fn text(&self) -> Option<String> {
        match self {
            ContentDelta::TextDelta { text } => Some(text.clone()),
            _ => None,
        }
    }

    pub fn bytes(&self) -> Vec<u8> {
        match self {
            ContentDelta::TextDelta { text } => text.as_bytes().to_vec(),
            _ => vec![],
        }
    }
}
