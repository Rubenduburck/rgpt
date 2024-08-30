use rgpt_types::message::Message;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub messages: Option<Vec<Message>>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub stream: bool,
    pub mode: Mode,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            messages: None,
            model: None,
            temperature: None,
            stream: true,
            mode: Mode::General,
        }
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Dev,
    Bash,
    #[default]
    General,
}

impl From<&str> for Mode {
    fn from(mode: &str) -> Self {
        match mode {
            "dev" => Mode::Dev,
            "bash" => Mode::Bash,
            _ => Mode::General,
        }
    }
}

impl Mode {
    pub fn config(&self) -> Config {
        match self {
            Mode::Dev => dev_config(),
            Mode::Bash => bash_config(),
            Mode::General => general_config(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Builder {
    mode: Mode,
    messages: Vec<Message>,
    model: Option<String>,
    temperature: Option<f32>,
    stream: Option<bool>,
}

impl Builder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self.messages = mode.config().messages.unwrap_or_default();
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
        self.stream = Some(stream);
        self
    }

    pub fn build(self) -> Config {
        Config {
            messages: Some(self.messages),
            model: self.model,
            temperature: self.temperature,
            stream: self.stream.unwrap_or(true),
            mode: self.mode,
        }
    }
}

impl Config {
    pub fn builder() -> Builder {
        Builder::new()
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
                If there is a lack of details, provide most logical solution.
                Ensure the output is a valid shell command.
                Never ever respond with something other than a shell command.
                Provide only plain text without markdown formatting.
                Do not provide formatting such as ```.
                If multiple steps required, try to combine them together using &&.
                If multiple options are possible, separate them with a newline.
                If a command requires a newline, use a backslash at the end of the line.
                User's `uname`: {}. User's `$SHELL`: {}.",
                std::env::consts::OS,
                std::env::var("SHELL").unwrap_or_else(|_| "Unknown".to_string())),
            },
        ]),
        ..Default::default()
    }
}
