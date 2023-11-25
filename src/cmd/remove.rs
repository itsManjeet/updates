use clap::{arg, value_parser, ArgMatches, Command};
use console::{style, Emoji};
use swupd::{engine::Engine, meta::MetaInfo};

use super::ask::ask;
pub use swupd::engine::Error;

static LOOKING_GLASS: Emoji<'_, '_> = Emoji("🔍  ", "");

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

    println!(
        "{} {}Resolving packages...",
        style("[2/4]").bold().dim(),
        LOOKING_GLASS
    );

    let mut to_remove: Vec<MetaInfo> = Vec::new();
    for package in packages.iter() {
        match engine.get(package, swupd::engine::ListMode::Installed) {
            Some(p) => to_remove.push(p),
            None => println!(
                "{} {} is not already installed",
                style("ERROR").red().bright(),
                package
            ),
        }
    }
    if to_remove.len() == 0 {
        return Ok(());
    }

    if !args.get_flag("yes") {
        for (position, package) in packages.iter().enumerate() {
            print!("{}. {}\t", position + 1, package);
        }
        println!();
        if !args.get_flag("yes") {
            if !ask(&format!("Do you want to remove above packages [y/N]: ",)) {
                return Ok(());
            }
        }
    }

    engine.remove(&packages).await
}
