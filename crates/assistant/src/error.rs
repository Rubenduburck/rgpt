#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("No api key")]
    NoApiKey,

    #[error("Provider error: {0}")]
    Provider(#[from] rgpt_provider::error::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Input error")]
    SendInput,

    #[error("Output error")]
    SendOutput,

    #[error("Draw error {0}")]
    Draw(String),

    #[error("Exit")]
    Exit,

    #[error("Join error")]
    Join(#[from] tokio::task::JoinError),

    #[error("State error")]
    State,

    #[error("Dialoguer error")]
    Dialoguer(#[from] dialoguer::Error),
}
