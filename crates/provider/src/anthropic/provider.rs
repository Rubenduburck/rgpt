use crate::anthropic::error::Error;
use crate::anthropic::types::{CompleteRequest, CompleteResponse, CompleteResponseStream};
use crate::anthropic::{API_BASE, API_VERSION, API_VERSION_HEADER_KEY, AUTHORIZATION_HEADER_KEY};
use reqwest::header::{HeaderMap, ACCEPT, CONTENT_TYPE};

use rgpt_caller::client::Client;

use super::types::{MessagesRequest, MessagesResponse, MessagesResponseStream};

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
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        headers.insert(ACCEPT, "application/json".parse().unwrap());
        headers.insert(API_VERSION_HEADER_KEY, API_VERSION.parse().unwrap());
        let caller = Client::new(headers);
        Self { api_key, caller }
    }

    pub async fn messages(&self, request: MessagesRequest) -> Result<MessagesResponse, Error> {
        Ok(self
            .caller
            .post(&format!("{}/v1/messages", API_BASE), request)
            .await?)
    }

    pub async fn messages_stream(
        &self,
        request: MessagesRequest,
    ) -> Result<MessagesResponseStream, Error> {
        Ok(self
            .caller
            .post_stream(&format!("{}/v1/messages", API_BASE), request)
            .await)
    }

    pub async fn complete(&self, request: CompleteRequest) -> Result<CompleteResponse, Error> {
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

    pub async fn complete_stream(
        &self,
        request: CompleteRequest,
    ) -> Result<CompleteResponseStream, Error> {
        if !request.stream {
            return Err(Error::InvalidArgument(
                "When stream is false, use complete() instead".into(),
            ));
        }
        Ok(self
            .caller
            .post_stream(&format!("{}/v1/complete", API_BASE), request)
            .await)
    }
}

#[cfg(test)]
mod tests {
    use crate::anthropic::types::Message;

    use super::super::{AI_PROMPT, HUMAN_PROMPT};
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
