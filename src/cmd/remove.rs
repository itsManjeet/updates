use clap::{arg, value_parser, ArgMatches, Command};
use swupd::engine::Engine;

pub use swupd::engine::Error;

use super::ask::ask;

pub fn cmd() -> Command {
    Command::new("remove")
        .about("Remove component into system")
        .arg(arg!(<NAME> ... "component to remove").value_parser(value_parser!(String)))
}

pub async fn run(args: &ArgMatches, engine: &mut Engine) -> Result<(), Error> {
    let packages = args
        .get_many::<String>("NAME")
        .into_iter()
        .flatten()
        .map(String::clone)
        .collect::<Vec<_>>();

    engine.sync().await?;

    if !ask(&format!(
        "{:?}\nDo you want to remove above packages [y/N]: ",
        packages
    )) {
        return Ok(());
    }
    engine.remove(&packages).await
}
