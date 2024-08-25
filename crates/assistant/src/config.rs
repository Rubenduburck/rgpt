use std::collections::HashMap;

use rgpt_types::completion::Message;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub messages: Option<Vec<Message>>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
}

lazy_static::lazy_static! {
    static ref DEFAULT_ASSISTANTS: HashMap<&'static str, Config> = {
        let mut m = HashMap::new();
        m.insert("dev", Config {
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
            model: None,
            temperature: None,
            top_p: None,
        });
        m.insert("general", Config {
            messages: Some(vec![]),
            model: None,
            temperature: None,
            top_p: None,
        });
        m.insert("bash", Config {
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
            model: None,
            temperature: None,
            top_p: None,
        });
        m
    };
}
