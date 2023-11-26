use std::{path::PathBuf, time::Instant};

use clap::{arg, value_parser, Arg, ArgAction, ArgMatches, Command};
use console::{style, Emoji};
use indicatif::HumanDuration;
pub use swupd::engine::Error;
use swupd::{element::Element, engine::Engine};

use super::ask::ask;

static LOOKING_GLASS: Emoji<'_, '_> = Emoji("🔍  ", "");
static TICK: Emoji<'_, '_> = Emoji("✔️  ", "");
static CLOUD: Emoji<'_, '_> = Emoji("☁️   ", "");
static SPARKLE: Emoji<'_, '_> = Emoji("✨ ", ":-)");

pub fn cmd() -> Command {
    Command::new("build")
        .about("Build element from source file")
        .arg(arg!(<NAME> ... "element file").value_parser(value_parser!(PathBuf)))
}

pub async fn run(args: &ArgMatches, engine: &mut Engine) -> Result<(), Error> {
    let element_path = args.get_one::<PathBuf>("NAME").unwrap();
    let started = Instant::now();

    println!(
        "{} {}Syncing Repository...",
        style("[2/4]").bold().dim(),
        CLOUD
    );
    engine.sync().await?;

    let mut element = Element::open(element_path, None)?;

    let mut required_packages: Vec<String> = Vec::new();
    if let Some(depends) = &mut element.depends {
        required_packages.append(depends);
    }

    if let Some(depends) = &mut element.build_depends {
        required_packages.append(depends);
    }

    println!(
        "{} {}Resolving packages...",
        style("[3/4]").bold().dim(),
        LOOKING_GLASS
    );

    let mut to_packages = engine.resolve(&required_packages).await?;
    if to_packages.len() == 0 {
        if !args.get_flag("force") {
            println!(
                "{} {}Packages already installed!",
                style("[4/4]").bold().dim(),
                TICK
            );
            return Ok(());
        } else {
            to_packages = required_packages
                .iter()
                .map(|s| engine.get(s, swupd::engine::ListMode::Remote).unwrap())
                .collect();
        }
    }
    let packages = to_packages;

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

    engine
        .install(&packages, args.get_flag("skip-integration"))
        .await?;
    println!(
        "{} Successfully installed {} package(s) in {}",
        SPARKLE,
        packages.len(),
        HumanDuration(started.elapsed())
    );

    Ok(())
}
