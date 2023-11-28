use clap::{arg, value_parser, Arg, ArgAction, ArgMatches, Command};
use console::{style, Emoji};
use indicatif::HumanDuration;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::{collections::hash_map::DefaultHasher, path::PathBuf, time::Instant};
use swupd::engine;
pub use swupd::engine::Error;
use swupd::{element::Element, engine::Engine};

use super::ask::ask;

static LOOKING_GLASS: Emoji<'_, '_> = Emoji("🔍  ", "");
static TICK: Emoji<'_, '_> = Emoji("✔️  ", "");
static CLOUD: Emoji<'_, '_> = Emoji("☁️   ", "");
static SPARKLE: Emoji<'_, '_> = Emoji("✨ ", ":-)");

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    environ: HashMap<String, String>,
    variables: HashMap<String, String>,
}

pub fn cmd() -> Command {
    Command::new("build")
        .about("Build element from source file")
        .arg(arg!(<NAME> ... "element file").value_parser(value_parser!(PathBuf)))
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .help("configuration file path")
                .action(ArgAction::Set)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("build-dir")
                .short('B')
                .long("build-dir")
                .help("path to perform build operation")
                .action(ArgAction::Set)
                .value_parser(value_parser!(PathBuf)),
        )
}

pub async fn run(args: &ArgMatches, engine: &mut Engine) -> Result<(), Error> {
    let element_path = args.get_one::<PathBuf>("NAME").unwrap();
    let started = Instant::now();
    let build_dir = match args.get_one::<PathBuf>("build-dir") {
        Some(build_dir) => build_dir.clone(),
        None => env::current_dir()?.join("build"),
    };
    let config = match args.get_one::<PathBuf>("config") {
        Some(config) => {
            let reader = File::open(config)?;
            let config: Config = serde_yaml::from_reader(reader)?;

            config
        }
        None => Config {
            variables: HashMap::new(),
            environ: HashMap::new(),
        },
    };

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

    let packages = engine.resolve(&required_packages).await?;

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
    println!(
        "{} Successfully installed {} package(s) in {}",
        SPARKLE,
        packages.len(),
        HumanDuration(started.elapsed())
    );

    let mut hasher = DefaultHasher::new();
    for package in packages.iter() {
        package.hash(&mut hasher);
    }
    let element = element.resolve(&config.environ, &config.variables)?;
    element.hash(&mut hasher);

    let hash = hasher.finish();

    let package_path =
        engine::builder::build(&element, &build_dir, &config.variables, &hash.to_string(), None).await?;

    println!("package is ready at {:?}", package_path);
    Ok(())
}
