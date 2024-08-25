
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Anthropic error: {0}")]
    Anthropic(#[from] crate::anthropic::error::Error),
}
