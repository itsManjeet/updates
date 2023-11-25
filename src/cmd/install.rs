use std::time::Instant;

use clap::{arg, value_parser, ArgMatches, Command};
use console::{style, Emoji};
use indicatif::HumanDuration;
use swupd::engine::Engine;
pub use swupd::engine::Error;

use super::ask::ask;

static LOOKING_GLASS: Emoji<'_, '_> = Emoji("🔍  ", "");
static TICK: Emoji<'_, '_> = Emoji("✔️  ", "");
static CLOUD: Emoji<'_, '_> = Emoji("☁️   ", "");
static SPARKLE: Emoji<'_, '_> = Emoji("✨ ", ":-)");

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

    let started = Instant::now();

    println!(
        "{} {}Syncing Repository...",
        style("[2/4]").bold().dim(),
        CLOUD
    );
    engine.sync().await?;

    println!(
        "{} {}Resolving packages...",
        style("[3/4]").bold().dim(),
        LOOKING_GLASS
    );

    let packages = engine.resolve(&packages).await?;
    if packages.len() == 0 {
        println!(
            "{} {}Packages already installed!",
            style("[4/4]").bold().dim(),
            TICK
        );
        return Ok(());
    }
    if packages.len() > 1 {
        println!("\nFound {} package(s) required", packages.len());
        for (position, package) in packages.iter().map(|i| i.id.clone()).enumerate() {
            print!("{}. {}\t", position + 1, package);
        }
        println!();

        if !ask(&format!("\nDo you want to install above packages [y/N]: ")) {
            return Ok(());
        }
    }
    engine.install(&packages).await?;
    println!("{} Done in {}", SPARKLE, HumanDuration(started.elapsed()));

    Ok(())
}
