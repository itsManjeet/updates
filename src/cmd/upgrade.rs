use std::time::Instant;

use clap::{ArgMatches, Command};
use console::{style, Emoji};
use indicatif::HumanDuration;
use swupd::engine::Engine;
pub use swupd::engine::Error;

use super::ask::ask;

static LOOKING_GLASS: Emoji<'_, '_> = Emoji("🔍  ", "");
static TICK: Emoji<'_, '_> = Emoji("✔️   ", "");
static CLOUD: Emoji<'_, '_> = Emoji("☁️   ", "");
static SPARKLE: Emoji<'_, '_> = Emoji("✨ ", ":-)");

pub fn cmd() -> Command {
    Command::new("upgrade").about("Upgrade system packages")
}

pub async fn run(args: &ArgMatches, engine: &mut Engine) -> Result<(), Error> {
    let started = Instant::now();

    println!(
        "{} {}Syncing Repository...",
        style("[2/4]").bold().dim(),
        CLOUD
    );
    engine.sync().await?;

    println!(
        "{} {}Checking outdated packages...",
        style("[3/4]").bold().dim(),
        LOOKING_GLASS
    );

    let packages = engine.list(swupd::engine::ListMode::Outdated).await?;
    if packages.len() == 0 {
        println!(
            "{} {}System is upto date!",
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
        if !args.get_flag("yes") {
            if !ask(&format!("\nDo you want to install above packages [y/N]: ")) {
                return Ok(());
            }
        }
    }
    engine.install(&packages, false).await?;
    println!("{} Done in {}", SPARKLE, HumanDuration(started.elapsed()));

    Ok(())
}
