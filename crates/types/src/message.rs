use serde::{Deserialize, Serialize};

// Equivalent to TypedDict in Python
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl From<String> for Message {
    fn from(content: String) -> Self {
        Self {
            role: "user".to_string(),
            content,
        }
    }
}
