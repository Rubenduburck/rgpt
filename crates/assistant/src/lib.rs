pub mod config;
pub mod error;
pub mod query;
pub mod session;
pub mod pagetree;
pub mod textarea;

use std::sync::Arc;

use config::{Config, Mode};
use query::Query;
use rgpt_provider::{api_key::ApiKey, Provider};
use rgpt_types::{
    completion::{Request, RequestBuilder, TextEvent},
    message::Message,
};

use error::Error;
use session::Session;
use tokio_stream::StreamExt as _;

pub struct Assistant {
    config: Config,
    provider: Arc<Provider>,
}

impl std::fmt::Debug for Assistant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Assistant")
            .field("config", &self.config)
            .finish()
    }
}

impl Assistant {
    pub fn new(config: Config) -> Result<Self, Error> {
        let provider = Arc::new(ApiKey::get().ok_or(Error::NoApiKey)?.get_provider());
        Ok(Self { config, provider })
    }

    fn mode(&self) -> Mode {
        self.config.mode
    }

    fn init_messages(&self) -> Vec<Message> {
        self.config.messages.clone().unwrap_or_default()
    }

    fn build_request(&self, messages: Vec<Message>) -> Request {
        RequestBuilder::new()
            .messages(messages)
            .model(self.config.model.clone())
            .temperature(self.config.temperature)
            .stream(self.config.stream)
            .build()
    }

    fn complete(&self, messages: Vec<Message>, tx: tokio::sync::mpsc::Sender<TextEvent>) {
        tracing::trace!("not streaming");
        let request = self.build_request(messages);
        let provider = self.provider.clone();
        tokio::spawn(async move {
            let response = match provider.complete(request).await {
                Ok(response) => {
                    tracing::trace!("response: {:?}", response);
                    response
                }
                Err(e) => {
                    tracing::error!("error: {}", e);
                    return;
                }
            };
            for event in <Vec<TextEvent>>::from(response) {
                if (tx.send(event).await).is_err() {
                    tracing::error!("error: send output");
                }
            }
        });
    }

    fn complete_stream(&self, messages: Vec<Message>, tx: tokio::sync::mpsc::Sender<TextEvent>) {
        tracing::trace!("streaming");
        let request = self.build_request(messages);
        let provider = self.provider.clone();
        tokio::spawn(async move {
            let mut stream = provider.complete_stream(request).await?;
            while let Some(event) = stream.next().await {
                match event {
                    Ok(event) => {
                        tracing::trace!("event: {:?}", event);
                        if (tx.send(event).await).is_err() {
                            tracing::error!("error: send output");
                        }
                    }
                    Err(e) => {
                        tracing::error!("error: {}", e);
                        break;
                    }
                }
            }
            Ok::<(), Error>(())
        });
    }

    pub fn handle_input(&self, messages: Vec<Message>, tx: tokio::sync::mpsc::Sender<TextEvent>) {
        if self.config.stream {
            self.complete_stream(messages, tx);
        } else {
            self.complete(messages, tx);
        }
    }

    pub async fn session(self, messages: &[Message]) -> Result<(), Error> {
        Session::setup(self)?.start(messages).await
    }

    pub async fn query(self, messages: &[Message]) -> Result<(), Error> {
        let execute = self.mode() == Mode::Bash;
        Query::builder(self)
            .execute(execute)
            .build()
            .start(messages)
            .await
    }
}

#[cfg(test)]
mod tests {
    use rgpt_types::message::Role;

    use super::*;

    fn get_config() -> Config {
        Config {
            messages: Some(vec![
                Message {
                    role: Role::System,
                    content: "You are my testing assistant. Whatever you say, start with 'Testing: '".to_string(),
                },
                Message {
                    role: Role::User,
                    content: "Your responses must be short and concise. Do not include explanations unless asked.".to_string(),
                },
                Message {
                    role: Role::Assistant,
                    content: "Understood.".to_string(),
                },
            ]),
            ..Default::default()
        }
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_assistant() -> Result<(), Error> {
        let cfg = get_config();
        let assistant = Assistant::new(cfg).unwrap();
        let test_messages = vec![Message {
            role: Role::User,
            content: "Testing: Hello, world!".to_string(),
        }];
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        assistant.complete(test_messages, tx);
        println!("response: {:?}", rx.recv().await.unwrap());
        Ok(())
    }
}
