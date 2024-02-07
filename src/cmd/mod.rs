use std::path::PathBuf;

use clap::{value_parser, Arg, ArgAction, Command};
use crate::{engine::Engine, Error};

mod list;
mod status;
mod unlock;
mod update;

pub async fn run() -> Result<(), Error> {
    let matches = Command::new("updates")
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
                .help("Ostree Sysroot")
                .default_value("/")
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("remote")
                .long("remote")
                .help("Specify remote or url")
                .action(ArgAction::Set)
                .global(true)
                .required(false)
                .value_parser(value_parser!(String)),
        )
        .arg_required_else_help(true)
        .subcommand(update::cmd())
        .subcommand(status::cmd())
        .subcommand(unlock::cmd())
        .subcommand(list::cmd())
        .get_matches();

    if matches.get_flag("version") {
        println!("version: {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if nix::unistd::getegid().as_raw() != 0 {
        return Err(Error::PermissionError(String::from(
            "need supper user access",
        )));
    }

    match unsafe { syscalls::syscall!(syscalls::Sysno::unshare, 0x00020000) } {
        Err(error) => return Err(Error::FailedSetupNamespace(error)),
        Ok(_) => {}
    };


    let engine = Engine::new(matches.get_one::<PathBuf>("sysroot").unwrap())?;


    match matches.subcommand() {
        Some(("update", args)) => update::run(args, &engine).await,
        Some(("status", args)) => status::run(args, &engine).await,
        Some(("unlock", args)) => unlock::run(args, &engine).await,
        Some(("list", args)) => list::run(args, &engine).await,
        _ => unreachable!(),
    }
}
