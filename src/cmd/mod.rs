use std::path::PathBuf;

use clap::{value_parser, Arg, ArgAction, Command};
use swupd::engine::{self, Engine};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Upgrade")]
    Upgrade(#[from] upgrade::Error),

    #[error("check")]
    Check(#[from] check::Error),

    #[error("status")]
    Status(#[from] status::Error),

    #[error("unlock")]
    Unlock(#[from] unlock::Error),

    #[error("engine")]
    Engine(#[from] engine::Error),

    #[error("permission error {0}")]
    PermissionError(String),
}

mod check;
mod status;
mod unlock;
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
                .global(true)
                .help("OStree Sysroot")
                .value_parser(value_parser!(PathBuf)),
        )
        .arg_required_else_help(true)
        .subcommand(upgrade::cmd())
        .subcommand(status::cmd())
        .subcommand(check::cmd())
        .subcommand(unlock::cmd())
        .get_matches();

    if matches.get_flag("version") {
        println!("version: {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let engine = Engine::new(matches.get_one::<PathBuf>("sysroot"));
    if nix::unistd::getegid().as_raw() != 0 {
        return Err(Error::PermissionError(String::from(
            "need supper user access",
        )));
    }
    Engine::setup_namespace()?;

    match matches.subcommand() {
        Some(("upgrade", args)) => upgrade::run(args, &engine).await.map_err(Error::Upgrade),
        Some(("check", args)) => check::run(args, &engine).await.map_err(Error::Check),
        Some(("status", args)) => status::run(args, &engine).await.map_err(Error::Status),
        Some(("unlock", args)) => unlock::run(args, &engine).await.map_err(Error::Unlock),
        _ => unreachable!(),
    }
}
