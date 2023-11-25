use clap::{arg, value_parser, ArgMatches, Command};
use console::{style, Emoji};
pub use swupd::engine::Error;
use swupd::engine::{Engine, ListMode};

static CLOUD: Emoji<'_, '_> = Emoji("☁️   ", "");
static LOOKING_GLASS: Emoji<'_, '_> = Emoji("🔍  ", "");
static TICK: Emoji<'_, '_> = Emoji("✔️  ", "");

pub fn cmd() -> Command {
    Command::new("search")
        .about("Search components from remote")
        .arg(arg!(<NAME> ... "info").value_parser(value_parser!(String)))
}

pub async fn run(args: &ArgMatches, engine: &mut Engine) -> Result<(), Error> {
    let info = args.get_one::<String>("NAME").unwrap();

    println!(
        "{} {}Syncing Repository...",
        style("[2/4]").bold().dim(),
        CLOUD
    );
    engine.sync().await?;

    println!(
        "{} {}Searching packages...",
        style("[3/4]").bold().dim(),
        LOOKING_GLASS
    );

    let found = engine.list(ListMode::Matched(info.to_string())).await?;
    if found.len() == 0 {
        println!("no components found");
        return Ok(());
    }

    println!(
        "{} {}Found {} component(s)!",
        style("[3/4]").bold().dim(),
        TICK,
        found.len(),
    );
    println!();
    for (idx, c) in found.iter().enumerate() {
        println!("{}. {}: {}", idx + 1, c.id, c.about);
    }
    Ok(())
}
