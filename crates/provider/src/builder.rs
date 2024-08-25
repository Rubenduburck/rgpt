use crate::api_key::ApiKey;
use crate::Provider;

pub struct Builder {
    api_key: ApiKey,
    model: Option<String>,
}

impl Builder {
    pub fn new(api_key: ApiKey) -> Self {
        Self {
            api_key,
            model: None,
        }
    }

    pub fn api_key(&mut self, api_key: ApiKey) -> &mut Self {
        self.api_key = api_key;
        self
    }

    pub fn model(&mut self, model: String) -> &mut Self {
        self.model = Some(model);
        self
    }

    pub fn build(self) -> Provider {
        self.api_key.get_provider()
    }
}
