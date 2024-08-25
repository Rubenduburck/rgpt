pub mod config;
pub mod error;
pub mod session;

use std::sync::Arc;

use config::Config;
use rgpt_provider::{api_key::ApiKey, Provider};
use rgpt_types::completion::{Message, Request, RequestBuilder, Response};

use error::Error;
use session::Session;
use tokio_stream::StreamExt as _;

pub struct Assistant {
    config: Config,
    provider: Arc<Provider>,
}

impl Assistant {
    pub fn new(config: Config) -> Result<Self, Error> {
        let provider = Arc::new(ApiKey::get().ok_or(Error::NoApiKey)?.get_provider());
        Ok(Self { config, provider })
    }

    fn init_messages(&self) -> Vec<Message> {
        self.config.messages.clone().unwrap_or_default()
    }

    fn build_request(&self, messages: Vec<Message>) -> Request {
        RequestBuilder::new()
            .messages(self.init_messages())
            .messages(messages)
            .model(self.config.model.clone())
            .temperature(self.config.temperature)
            .stream(self.config.stream)
            .build()
    }

    fn complete(&self, messages: Vec<Message>, tx: tokio::sync::mpsc::Sender<Response>) {
        let request = self.build_request(messages);
        let provider = self.provider.clone();
        tokio::spawn(async move {
            let response = provider.complete(request).await?;
            tx.send(response).await.map_err(|_| Error::SendOutput)?;
            Ok::<(), Error>(())
        });
    }

    fn complete_stream(&self, messages: Vec<Message>, tx: tokio::sync::mpsc::Sender<Response>) {
        let request = self.build_request(messages);
        let provider = self.provider.clone();
        tokio::spawn(async move {
            let stream = provider.complete_stream(request).await?;
            let mut stream = stream;
            while let Some(response) = stream.next().await {
                tracing::debug!("response: {:?}", response);
                tx.send(response?).await.map_err(|_| Error::SendOutput)?;
            }
            Ok::<(), Error>(())
        });
    }

    fn handle_input(&self, messages: Vec<Message>, tx: tokio::sync::mpsc::Sender<Response>) {
        if self.config.stream {
            self.complete_stream(messages, tx);
        } else {
            self.complete(messages, tx);
        }
    }

    pub async fn session(self, messages: &[Message]) -> Result<(), Error> {
        Session::start(self, messages).await
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_config() -> Config {
        Config {
            messages: Some(vec![
                Message {
                    role: "system".to_string(),
                    content: "You are my testing assistant. Whatever you say, start with 'Testing: '".to_string(),
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

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_assistant() -> Result<(), Error> {
        let cfg = get_config();
        let assistant = Assistant::new(cfg).unwrap();
        let test_messages = vec![Message {
            role: "user".to_string(),
            content: "Testing: Hello, world!".to_string(),
        }];
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        assistant.complete(test_messages, tx);
        println!("response: {:?}", rx.recv().await.unwrap());
        Ok(())
    }
}
