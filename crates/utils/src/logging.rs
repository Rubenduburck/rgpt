pub fn init_logger(filename: Option<&str>) {
    use tracing_subscriber::fmt::format::FmtSpan;
    use tracing_subscriber::prelude::*;

    let fmt_layer = if let Some(filename) = filename {
        let file = std::fs::File::create(filename).unwrap();
        tracing_subscriber::fmt::layer()
            .with_writer(file)
            .with_target(false)
            .with_span_events(FmtSpan::CLOSE)
    } else {
        tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_span_events(FmtSpan::CLOSE)
    };

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(fmt_layer)
        .init();
}
