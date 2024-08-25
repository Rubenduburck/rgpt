use std::pin::Pin;

use error::Error;
use rgpt_types::completion::{TextEvent, Request, Response};

use rgpt_utils::stream::adapt_stream;
use tokio_stream::Stream;

mod anthropic;
pub mod api_key;
pub mod builder;
pub mod error;

pub enum Provider {
    Anthropic(anthropic::provider::Provider),
}

pub type ResponseStream = Pin<Box<dyn Stream<Item = Result<Response, Error>> + Send>>;
pub type EventsStream = Pin<Box<dyn Stream<Item = Result<TextEvent, Error>> + Send>>;

impl Provider {
    pub async fn complete(&self, request: Request) -> Result<Response, Error> {
        Ok(match self {
            Self::Anthropic(provider) => provider.messages(request).await,
        }?
        .into())
    }

    pub async fn complete_stream(&self, request: Request) -> Result<EventsStream, Error> {
        let stream = match self {
            Self::Anthropic(provider) => provider.messages_stream(request).await,
        }?;
        Ok(adapt_stream(stream, |res| res.map(Into::into).map_err(Into::into)))
    }
}
