use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

impl From<&str> for Role {
    fn from(role: &str) -> Self {
        match role {
            "user" => Role::User,
            "assistant" => Role::Assistant,
            "system" => Role::System,
            _ => Role::User,
        }
    }
}

// Equivalent to TypedDict in Python
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl From<String> for Message {
    fn from(content: String) -> Self {
        Self {
            role: Role::User,
            content,
        }
    }
}
