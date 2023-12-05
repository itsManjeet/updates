use clap::{ArgMatches, Command};

use ostree::{gio::Cancellable, DeploymentUnlockedState};
use swupd::engine::{Engine, Error};


pub fn cmd() -> Command {
    Command::new("unlock").about("Add safe mutable overlay")
}

pub async fn run(_: &ArgMatches, engine: &Engine) -> Result<(), Error> {
    let cancellable = Cancellable::NONE;

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
