use clap::{value_parser, Arg, ArgAction, Command};
use console::{style, Emoji};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use std::fmt::Write;
use std::path::PathBuf;
use swupd::engine::Engine;
use thiserror::Error;

mod ask;
mod install;
mod remove;
mod search;

static TRUCK: Emoji<'_, '_> = Emoji("🚚  ", "");

pub async fn run() -> Result<(), Error> {
    let matches = Command::new("swupd")
        .about("Software Management and updater daemon")
        .arg(
            Arg::new("root")
                .short('D')
                .long("root")
                .help("Specify root path")
                .action(ArgAction::Set)
                .default_value("/")
                .global(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("server")
                .short('U')
                .long("server-url")
                .help("Specify server url")
                .action(ArgAction::Set)
                .default_value("http://repo.rlxos.dev")
                .global(true)
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("version")
                .short('v')
                .long("version")
                .action(ArgAction::SetTrue),
        )
        .arg_required_else_help(true)
        .subcommand(install::cmd())
        .subcommand(remove::cmd())
        .subcommand(search::cmd())
        .get_matches();

    if matches.get_flag("version") {
        println!("version: {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let mut engine = Engine::new(
        matches.get_one::<PathBuf>("root").unwrap(),
        matches.get_one::<String>("server").unwrap(),
    );
    let progress = ProgressBar::new(100);
    progress.set_style(
        ProgressStyle::with_template("{spinner:.green:5} {prefix:.bold.dim} {wide_msg}").unwrap(),
    );
    engine.set_progress(progress);

    println!(
        "{} {}Loading system state...",
        style("[1/4]").bold().dim(),
        TRUCK
    );
    engine.load().await?;

    match matches.subcommand() {
        Some(("install", args)) => install::run(args, &mut engine).await.map_err(Error::Swupd),
        Some(("remove", args)) => remove::run(args, &mut engine).await.map_err(Error::Swupd),
        Some(("search", args)) => search::run(args, &mut engine).await.map_err(Error::Swupd),
        _ => unreachable!(),
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Swupd")]
    Swupd(#[from] swupd::engine::Error),
}
