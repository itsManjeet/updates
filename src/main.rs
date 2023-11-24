use std::error::Error;

mod cmd;

#[tokio::main]
async fn main() {
    if let Err(error) = cmd::run().await {
        println!("ERROR: Failed to execute task");
        let mut source = error.source();
        while let Some(err) = source.take() {
            eprintln!("  TRACE: {}", err.to_string());
            source = err.source();
        }
    }
}
