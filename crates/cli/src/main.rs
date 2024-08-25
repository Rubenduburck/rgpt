use clap::Parser;

#[derive(Parser)]
struct Args {
    input: Option<String>,
}

impl Args {
    fn execute(&self) {
        tracing::debug!("Executing with input: {:?}", self.input);
    }
}

fn main() {
    rgpt_utils::logging::init_logger();
    Args::parse().execute();
}
