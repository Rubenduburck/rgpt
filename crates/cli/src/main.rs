pub mod error;

use clap::Parser;
use error::Error;
use rgpt_assistant::{config::Config, Assistant};
use rgpt_types::completion::Message;

#[derive(Parser, Debug)]
struct Args {
    #[clap(short, long)]
    session: bool,
    #[clap(short, long, default_value = "general")]
    mode: String,

    input: Option<String>,
}

impl Args {
    async fn execute(&self) -> Result<(), Error> {
        let cfg = Config::builder()
            .mode(&self.mode)
            .stream(true)
            .build();
        let messages = self
            .input
            .as_ref()
            .map_or_else(Vec::new, |input| vec![Message::from(input.clone())]);
        tracing::debug!("Starting assistant with config: {:?}", cfg);
        let assistant = Assistant::new(cfg)?;
        assistant.session(&messages).await?;
        Ok(())

    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    rgpt_utils::logging::init_logger();
    Args::parse().execute().await
}
