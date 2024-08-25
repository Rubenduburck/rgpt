

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Assistant error: {0}")]
    AssistantError(#[from] rgpt_assistant::error::Error),
}
