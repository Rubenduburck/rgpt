pub struct ApiKey {
    pub key: String,
}

impl From<String> for ApiKey {
    fn from(key: String) -> Self {
        Self { key }
    }
}

impl ApiKey {
    const API_KEY_ENV_VAR: &'static str = "ANTHROPIC_API_KEY";
    pub fn get() -> Option<Self> {
        get().map(Self::from)
    }
}

impl From<ApiKey> for String {
    fn from(api_key: ApiKey) -> String {
        api_key.key
    }
}

pub fn get() -> Option<String> {
    std::env::var(ApiKey::API_KEY_ENV_VAR).ok()
}
