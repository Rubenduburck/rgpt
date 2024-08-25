use anthropic::types::{CompleteRequest, CompleteResponse, CompleteResponseStream};
use error::Error;

mod anthropic;
pub mod error;
pub mod builder;
pub mod api_key;

pub enum Provider {
    Anthropic(anthropic::provider::Provider),
}

impl Provider {
    pub async fn complete(&self, request: CompleteRequest) -> Result<CompleteResponse, Error> {
        Ok(match self {
            Self::Anthropic(provider) => provider.complete(request).await,
        }?)
    }

    pub async fn complete_stream(
        &self,
        request: CompleteRequest,
    ) -> Result<CompleteResponseStream, Error> {
        Ok(match self {
            Self::Anthropic(provider) => provider.complete_stream(request).await,
        }?)
    }
}
