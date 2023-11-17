use std::path::PathBuf;

use clap::{value_parser, Arg, ArgAction, Command};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Upgrade")]
    Upgrade(#[from] upgrade::Error),
}

mod upgrade;

pub async fn run() -> Result<(), Error> {
    let matches = Command::new("swupd")
        .about("Software Updater daemon")
        .arg(
            Arg::new("version")
                .short('v')
                .long("version")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("sysroot")
                .long("sysroot")
                .default_value("/sysroot")
                .global(true)
                .help("OStree Sysroot")
                .value_parser(value_parser!(PathBuf)),
        )
        .arg_required_else_help(true)
        .subcommand(upgrade::cmd())
        .get_matches();

    if matches.get_flag("version") {
        println!("version: {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    match matches.subcommand() {
        Some(("upgrade", args)) => upgrade::run(args).await.map_err(Error::Upgrade),
        _ => unreachable!(),
    }
}
