use std::pin::Pin;

use crate::anthropic::error::Error;
use crate::anthropic::types::{CompleteRequest, CompleteResponse};
use crate::anthropic::{API_BASE, API_VERSION, API_VERSION_HEADER_KEY, AUTHORIZATION_HEADER_KEY};
use reqwest::header::{HeaderMap, ACCEPT, CONTENT_TYPE};

use reqwest_eventsource::Event;
use rgpt_caller::client::Client;
use tokio_stream::Stream;

use super::types::{MessagesEvent, MessagesRequest, MessagesResponse};
use super::{CLIENT_ID, CLIENT_ID_HEADER_KEY};

pub type MessagesEventStream =
    Pin<Box<dyn Stream<Item = Result<MessagesEvent, Error>> + Send>>;

#[derive(Debug)]
pub struct Provider {
    pub api_key: String,
    caller: Client,
}

impl Provider {
    pub fn new(api_key: String) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", api_key).parse().unwrap(),
        );
        headers.insert(AUTHORIZATION_HEADER_KEY, api_key.parse().unwrap());
        headers.insert(CLIENT_ID_HEADER_KEY, CLIENT_ID.parse().unwrap());
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        headers.insert(ACCEPT, "application/json".parse().unwrap());
        headers.insert(API_VERSION_HEADER_KEY, API_VERSION.parse().unwrap());
        let caller = Client::new(headers);
        Self { api_key, caller }
    }

    pub async fn messages<R>(&self, request: R) -> Result<MessagesResponse, Error>
    where
        R: Into<MessagesRequest>,
    {
        let request = request.into();
        if request.stream {
            return Err(Error::InvalidArgument(
                "When stream is true, use messages_stream() instead".into(),
            ));
        }
        tracing::debug!("request: {:?}", request);
        Ok(self
            .caller
            .post(&format!("{}/v1/messages", API_BASE), request)
            .await?)
    }

    pub async fn messages_stream<R>(&self, request: R) -> Result<MessagesEventStream, Error>
    where
        R: Into<MessagesRequest>,
    {
        let request = request.into();
        if !request.stream {
            return Err(Error::InvalidArgument(
                "When stream is false, use messages() instead".into(),
            ));
        }
        let stream = self
            .caller
            .post_stream(&format!("{}/v1/messages", API_BASE), request, Self::messages_handler)
            .await;
        Ok(stream?)
    }

    pub fn messages_handler(event: reqwest_eventsource::Event) -> Result<MessagesEvent, Error>
    {
        tracing::debug!("event: {:?}", event);
        match event{
            Event::Open => Ok(MessagesEvent::MessageOpen),
            Event::Message(message) => {
                match serde_json::from_str::<MessagesEvent>(&message.data){
                    Ok(event) => Ok(event),
                    Err(e) => {
                        tracing::error!("error deserializing event: {:?}", e);
                        Err(Error::JSONDeserialize(e))
                    }
                }
            },

        }
    }

    pub async fn complete<R>(&self, request: R) -> Result<CompleteResponse, Error>
    where
        R: Into<CompleteRequest>,
    {
        let request = request.into();
        if request.stream {
            return Err(Error::InvalidArgument(
                "When stream is true, use complete_stream() instead".into(),
            ));
        }
        Ok(self
            .caller
            .post(&format!("{}/v1/complete", API_BASE), request)
            .await?)
    }

    pub async fn complete_stream<R>(&self, request: R) -> Result<MessagesEventStream, Error>
    where
        R: Into<CompleteRequest>,
    {
        let request = request.into();
        if !request.stream {
            return Err(Error::InvalidArgument(
                "When stream is false, use complete() instead".into(),
            ));
        }
        let stream = self
            .caller
            .post_stream(&format!("{}/v1/complete", API_BASE), request, Self::complete_handler)
            .await;
        Ok(stream?)
    }

    pub fn complete_handler(event: reqwest_eventsource::Event) -> Result<MessagesEvent, Error> {
        match event {
            Event::Open => Ok(MessagesEvent::MessageOpen),
            Event::Message(message) => {
                let event = serde_json::from_str::<MessagesEvent>(&message.data)?;
                tracing::debug!("event: {:?}", event);
                Ok(event)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::anthropic::types::Message;

    const AI_PROMPT: &str = "Assistant: ";
    const HUMAN_PROMPT: &str = "Human: ";
    use super::*;

    #[tokio::test]
    async fn test_complete() -> Result<(), Box<dyn std::error::Error>> {
        let prompt = format!("{HUMAN_PROMPT}A human walks into a bar{AI_PROMPT}");

        // get the api key from the environment
        let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap();
        let client = Provider::new(api_key);
        let request = CompleteRequest {
            prompt,
            ..Default::default()
        };

        let response = client.complete(request).await.unwrap();
        println!("response: {:?}", response);
        Err("test not implemented".into())
    }

    #[tokio::test]
    async fn test_messages() -> Result<(), Box<dyn std::error::Error>> {
        let messages = vec![
            Message {
                role: "user".into(),
                content: "A human walks into a bar".into(),
            },
            Message {
                role: "assistant".into(),
                content: "The bartender says, 'What can I get you?'".into(),
            },
            Message {
                role: "user".into(),
                content: "The human says, 'I'll have a beer'".into(),
            },
        ];

        // get the api key from the environment
        let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap();
        let client = Provider::new(api_key);
        let request = MessagesRequest {
            messages,
            ..Default::default()
        };

        let response = client.messages(request).await.unwrap();
        println!("response: {:?}", response);
        Err("test not implemented".into())
    }
}
