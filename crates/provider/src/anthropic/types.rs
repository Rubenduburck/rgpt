use rgpt_types::completion::{Request, TextEvent};
use serde::{Deserialize, Serialize};

use crate::anthropic::DEFAULT_MODEL;

use super::DEFAULT_MAX_TOKENS;

#[derive(Debug, Deserialize, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    MaxTokens,
    StopSequence,
    EndTurn,
}

impl From<StopReason> for rgpt_types::completion::StopReason {
    fn from(reason: StopReason) -> Self {
        match reason {
            StopReason::MaxTokens => Self::MaxTokens,
            StopReason::StopSequence => Self::StopSequence,
            StopReason::EndTurn => Self::EndTurn,
        }
    }
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

// Messages API
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl From<rgpt_types::message::Message> for Message {
    fn from(message: rgpt_types::message::Message) -> Self {
        Self {
            role: message.role,
            content: message.content,
        }
    }
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

impl From<Request> for MessagesRequest {
    fn from(val: Request) -> Self {
        let (system, messages) =
            val.messages
                .into_iter()
                .fold((None, vec![]), |(system, mut messages), message| {
                    if message.role == "system" {
                        (Some(message.content), messages)
                    } else {
                        messages.push(message.into());
                        (system, messages)
                    }
                });
        MessagesRequest {
            messages,
            model: val.model.unwrap_or(DEFAULT_MODEL.to_string()),
            max_tokens: val.max_tokens,
            stop_sequences: val.stop_sequences,
            stream: val.stream,
            system,
            temperature: val.temperature,
        }
    }
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
#[serde(rename_all = "snake_case")]
pub enum Content {
    Text(String),
    Other,
}

impl From<Content> for rgpt_types::completion::Content {
    fn from(content: Content) -> Self {
        match content {
            Content::Text(text) => Self::Text(text),
            Content::Other => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Usage {
    input_tokens: usize,
    output_tokens: usize,
}

impl From<Usage> for rgpt_types::completion::Usage {
    fn from(usage: Usage) -> Self {
        Self {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
        }
    }
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

impl From<MessagesResponse> for rgpt_types::completion::Response {
    fn from(response: MessagesResponse) -> Self {
        Self {
            stop_reason: response
                .stop_reason
                .map(rgpt_types::completion::StopReason::from),
            stop_sequence: response.stop_sequence,
            content: response
                .content
                .into_iter()
                .map(rgpt_types::completion::Content::from)
                .collect(),
            model: response.model,
            id: response.id,
            type_: response.type_,
            usage: response.usage.into(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum MessagesEvent {
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

impl From<MessagesEvent> for TextEvent {
    fn from(event: MessagesEvent) -> Self {
        match event {
            MessagesEvent::Ping => TextEvent::Null,
            MessagesEvent::MessageStart { message } => TextEvent::MessageStart {
                message: message.into(),
            },
            MessagesEvent::MessageOpen => TextEvent::Null,
            MessagesEvent::ContentBlockStop { index } => TextEvent::ContentBlockStop { index },
            MessagesEvent::ContentBlockStart {
                index,
                content_block,
            } => TextEvent::ContentBlockStart {
                index,
                content_block: content_block.into(),
            },
            MessagesEvent::ContentBlockDelta { index, delta } => TextEvent::ContentBlockDelta {
                index,
                delta: delta.into(),
            },
            MessagesEvent::MessageDelta { delta } => TextEvent::MessageDelta {
                delta: delta.into(),
            },
            MessagesEvent::MessageStop => TextEvent::MessageStop,
        }
    }
}

impl From<ContentBlock> for rgpt_types::completion::ContentBlock {
    fn from(content_block: ContentBlock) -> Self {
        match content_block {
            ContentBlock::Text { text } => Self::Text { text },
        }
    }
}

impl From<Delta> for rgpt_types::completion::ContentDelta {
    fn from(delta: Delta) -> Self {
        match delta {
            Delta::TextDelta { text } => Self::TextDelta { text },
        }
    }
}

impl From<MessageDelta> for rgpt_types::completion::MessageDelta {
    fn from(delta: MessageDelta) -> Self {
        Self {
            stop_reason: delta
                .stop_reason
                .map(rgpt_types::completion::StopReason::from),
            stop_sequence: delta.stop_sequence,
        }
    }
}

impl From<MessageStartData> for rgpt_types::completion::MessageStartData {
    fn from(data: MessageStartData) -> Self {
        Self {
            id: data.id,
            type_: data.type_,
            role: data.role,
            model: data.model,
            content: data
                .content
                .into_iter()
                .map(rgpt_types::completion::Content::from)
                .collect(),
            stop_reason: data
                .stop_reason
                .map(rgpt_types::completion::StopReason::from),
            stop_sequence: data.stop_sequence,
            usage: data.usage.into(),
        }
    }
}
