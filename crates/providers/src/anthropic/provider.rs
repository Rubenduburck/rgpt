use crate::anthropic::error::Error;
use crate::anthropic::types::{CompleteRequest, CompleteResponse, CompleteResponseStream};
use crate::anthropic::{
    API_VERSION, API_VERSION_HEADER_KEY, AUTHORIZATION_HEADER_KEY, DEFAULT_API_BASE, DEFAULT_MODEL,
};
use reqwest::header::{HeaderMap, ACCEPT, CONTENT_TYPE};

use rgpt_caller::client::Client;

#[derive(Debug)]
pub struct Provider {
    pub api_key: String,
    pub api_base: String,
    pub default_model: String,
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
        Self {
            api_key,
            api_base: DEFAULT_API_BASE.to_string(),
            default_model: DEFAULT_MODEL.to_string(),
            caller,
        }
    }

    pub async fn complete(&self, request: CompleteRequest) -> Result<CompleteResponse, Error> {
        if request.stream {
            return Err(Error::InvalidArgument(
                "When stream is true, use complete_stream() instead".into(),
            ));
        }
        Ok(self
            .caller
            .post(&format!("{}/v1/complete", self.api_base), request)
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
            .post_stream(&format!("{}/v1/complete", self.api_base), request)
            .await)
    }
}

#[cfg(test)]
mod tests {
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
            model: "claude-v1".into(),
            max_tokens_to_sample: 256,
            stop_sequences: vec![HUMAN_PROMPT.to_string()].into(),
            stream: false,
        };

        let response = client.complete(request).await.unwrap();
        println!("response: {:?}", response);
        Err("test not implemented".into())
    }
}
