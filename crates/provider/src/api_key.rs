pub enum ApiKey {
    Anthropic(crate::anthropic::api_key::ApiKey),
}

impl ApiKey {
    pub fn get() -> Option<Self> {
        crate::anthropic::api_key::ApiKey::get().map(Self::Anthropic)
    }

    pub fn get_provider(&self) -> crate::Provider {
        match self {
            Self::Anthropic(key) => crate::Provider::Anthropic(
                crate::anthropic::provider::Provider::new(key.key.clone()),
            ),
        }
    }
}
