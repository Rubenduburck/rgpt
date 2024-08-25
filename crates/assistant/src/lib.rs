pub mod config;
pub mod error;
use config::Config;
use rgpt_provider::{api_key::ApiKey, Provider};
use rgpt_types::completion::{CompleteRequest, CompletionEvent, Message, ModelOverrides};

use error::Error;

struct Assistant {
    config: Config,
    provider: Provider,
}

impl Assistant {
    fn new(config: Config) -> Result<Self, Error> {
        let provider = ApiKey::get().ok_or(Error::NoApiKey)?.get_provider();
        Ok(Self { config, provider })
    }

    fn init_messages(&self) -> Vec<Message> {
        self.config.messages.clone().unwrap_or_default()
    }

    fn supported_overrides(&self) -> Vec<String> {
        vec![
            "model".to_string(),
            "temperature".to_string(),
            "top_p".to_string(),
        ]
    }

    //fn build_request(&self, messages: Vec<Message>, overrides: ModelOverrides) -> CompleteRequest {
    //    let mut request = CompleteRequest {
    //        messages,
    //        max_tokens: self.config.max_tokens,
    //        stop_sequences: self.config.stop_sequences.clone(),
    //        stream: self.config.stream,
    //        ..Default::default()
    //    };
    //    for (key, value) in overrides {
    //        match key.as_str() {
    //            "model" => request.model = value,
    //            "temperature" => request.temperature = value,
    //            "top_p" => request.top_p = value,
    //            _ => {}
    //        }
    //    }
    //    request
    //}

    //async fn complete(&self, messages: Vec<Message>, overrides: ModelOverrides) -> CompletionEvent {
    //    self.provider.complete().await
    //}
}
