
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("No api key")]
    NoApiKey,
}
