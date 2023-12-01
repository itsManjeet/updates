use std::path::PathBuf;

use clap::{value_parser, Arg, ArgAction, Command};
use ostree::{gio, glib, Sysroot};
use ostree::gio::Cancellable;
use swupd::engine::{self, Engine};
use thiserror::Error;


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
                .help("Ostree Sysroot")
                .default_value("/")
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

    let sysroot = Sysroot::new(Some(&gio::File::for_path(matches.get_one::<PathBuf>("sysroot").unwrap())));
    if nix::unistd::getegid().as_raw() != 0 {
        return Err(Error::PermissionError(String::from(
            "need supper user access",
        )));
    }

    sysroot.set_mount_namespace_in_use();
    sysroot.load(Cancellable::NONE)?;

    if !sysroot.try_lock()? {
        return Err(Error::FailedTryLock);
    }

    sysroot.connect_journal_msg(|_, msg| {
        println!("{}", msg);
    });

    let engine = Engine::new(sysroot, None)?;

    match unsafe { syscalls::syscall!(syscalls::Sysno::unshare, 0x00020000) } {
        Err(error) => return Err(Error::FailedSetupNamespace(error)),
        Ok(_) => {}
    };


    match matches.subcommand() {
        Some(("upgrade", args)) => upgrade::run(args, &engine).await.map_err(Error::Engine),
        Some(("check", args)) => check::run(args, &engine).await.map_err(Error::Engine),
        Some(("status", args)) => status::run(args, &engine).await.map_err(Error::Engine),
        Some(("unlock", args)) => unlock::run(args, &engine).await.map_err(Error::Unlock),
        _ => unreachable!(),
    }
}


#[derive(Debug, Error)]
pub enum Error {
    #[error("unlock")]
    Unlock(#[from] unlock::Error),

    #[error("engine")]
    Engine(#[from] engine::Error),

    #[error("glib")]
    GLib(#[from] glib::Error),

    #[error("permission error {0}")]
    PermissionError(String),

    #[error("failed to lock sysroot")]
    FailedTryLock,

    #[error("failed to setup namespace {0}")]
    FailedSetupNamespace(syscalls::Errno),
}
