use std::error::Error;

mod cmd;

#[tokio::main]
async fn main() {
    if let Err(error) = cmd::run().await {
        let error: cmd::Error = error;
        let mut source = error.source();
        eprint!("ERROR");
        while let Some(e) = source.take() {
            eprint!("::{e}");
            source = e.source();
        }
        eprintln!();

        std::process::exit(1);
    }
}
