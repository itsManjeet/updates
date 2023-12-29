use clap::{ArgMatches, Command};

use ostree::{gio::Cancellable, DeploymentUnlockedState};
use updatectl::engine::{self, Engine};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("GLib Error")]
    GLibError(#[from] ostree::glib::Error),

    #[error("Engine")]
    Engine(#[from] engine::Error),
}

pub fn cmd() -> Command {
    Command::new("unlock").about("Add safe mutable overlay")
}

pub async fn run(_: &ArgMatches, engine: &Engine) -> Result<(), Error> {
    let cancellable = Cancellable::NONE;

    engine.load(cancellable)?;
    if let Some(deployment) = engine.sysroot.booted_deployment() {
        if deployment.unlocked() != DeploymentUnlockedState::None {
            println!("safe mutable overlay already applied");
            return Ok(());
        }
        engine.sysroot.deployment_unlock(
            &deployment,
            ostree::DeploymentUnlockedState::Development,
            cancellable,
        )?;
        println!("applied safe mutable overlay");
    } else {
        println!("not booted with ostree");
    }

    Ok(())
}
