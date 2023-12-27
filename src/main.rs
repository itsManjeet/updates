use std::error::Error;
use updatectl::engine;

mod cmd;

#[tokio::main]
async fn main() {
    if let Err(error) = cmd::run().await {
        report_error(error);
        std::process::exit(1);
    }
}

fn report_error(error: engine::Error) {
    let sources = sources(&error);
    let error = sources.join(": ");
    eprintln!("ERROR: {error}");
}

fn sources(error: &engine::Error) -> Vec<String> {
    let mut sources = vec![error.to_string()];
    let mut source = error.source();
    while let Some(error) = source.take() {
        sources.push(error.to_string());
        source = error.source();
    }
    sources
}
