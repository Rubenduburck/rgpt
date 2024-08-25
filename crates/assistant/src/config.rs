use rgpt_types::message::Message;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Config {
    pub messages: Option<Vec<Message>>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub stream: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Builder {
    messages: Vec<Message>,
    model: Option<String>,
    temperature: Option<f32>,
    stream: bool,
}

impl Builder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn mode(mut self, mode: &str) -> Self {
        self.messages = get_assistant(mode).messages.unwrap_or_default();
        self
    }

    pub fn messages(mut self, messages: Vec<Message>) -> Self {
        self.messages.extend(messages);
        self
    }

    pub fn model(mut self, model: Option<String>) -> Self {
        self.model = model;
        self
    }

    pub fn temperature(mut self, temperature: Option<f32>) -> Self {
        self.temperature = temperature;
        self
    }

    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    pub fn build(self) -> Config {
        Config {
            messages: Some(self.messages),
            model: self.model,
            temperature: self.temperature,
            stream: self.stream,
        }
    }
}

impl Config {
    pub fn builder() -> Builder {
        Builder::new()
    }
}

fn get_assistant(mode: &str) -> Config {
    match mode {
        "dev" => dev_config(),
        "bash" => bash_config(),
        _ => general_config(),
    }
}

fn dev_config() -> Config {
    Config {
        messages: Some(vec![
            Message {
                role: "system".to_string(),
                content: format!("You are a helpful assistant who is an expert in software development. \
                You are helping a user who is a software developer. Your responses are short and concise. \
                You include code snippets when appropriate. Code snippets are formatted using Markdown \
                with a correct language tag. User's `uname`: {}", std::env::consts::OS),
            },
            Message {
                role: "user".to_string(),
                content: "Your responses must be short and concise. Do not include explanations unless asked.".to_string(),
            },
            Message {
                role: "assistant".to_string(),
                content: "Understood.".to_string(),
            },
        ]),
        ..Default::default()
    }
}

pub fn general_config() -> Config {
    Config {
        ..Default::default()
    }
}

fn bash_config() -> Config {
    Config {
        messages: Some(vec![
            Message {
                role: "system".to_string(),
                content: format!("You output only valid and correct shell commands according to the user's prompt. \
                You don't provide any explanations or any other text that is not valid shell commands. \
                User's `uname`: {}. User's `$SHELL`: {}.",
                std::env::consts::OS,
                std::env::var("SHELL").unwrap_or_else(|_| "Unknown".to_string())),
            },
        ]),
        ..Default::default()
    }
}
