use clap::{arg, value_parser, ArgMatches, Command};
use swupd::engine::Engine;

pub use swupd::engine::Error;

use super::ask::ask;

pub fn cmd() -> Command {
    Command::new("install")
        .about("Install component into system")
        .arg(arg!(<NAME> ... "component to install").value_parser(value_parser!(String)))
}

pub async fn run(args: &ArgMatches, engine: &mut Engine) -> Result<(), Error> {
    let packages = args
        .get_many::<String>("NAME")
        .into_iter()
        .flatten()
        .map(String::clone)
        .collect::<Vec<_>>();

    engine.sync().await?;

    if let Some(progress) = &engine.progress {
        progress.set_message("RESOLVING DEPENDENCIES");
    }
    let packages = engine.resolve(&packages).await?;
    if packages.len() == 0 {
        if let Some(progress) = &engine.progress {
            progress.finish_with_message("Packages are already installed");
        }
        return Ok(());
    }
    if packages.len() > 1 {
        let packages_id: Vec<String> = packages.iter().map(|i| i.id.clone()).collect();
        if !ask(&format!(
            "{:?}\nDo you want to install above packages [y/N]: ",
            packages_id
        )) {
            return Ok(());
        }
    }
    engine.install(&packages).await
}
