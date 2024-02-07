use clap::{ArgMatches, Command};

use crate::{engine::Engine, Error};

pub fn cmd() -> Command {
    Command::new("unlock").about("Add safe mutable overlay")
}

pub async fn run(_: &ArgMatches, engine: &Engine) -> Result<(), Error> {
    engine.add_overlay()
}
