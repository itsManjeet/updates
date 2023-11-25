use clap::{arg, value_parser, ArgMatches, Command};
use swupd::engine::{Engine, ListMode};

pub use swupd::engine::Error;

pub fn cmd() -> Command {
    Command::new("search")
        .about("Search components from remote")
        .arg(arg!(<NAME> ... "info").value_parser(value_parser!(String)))
}

pub async fn run(args: &ArgMatches, engine: &mut Engine) -> Result<(), Error> {
    let info = args.get_one::<String>("NAME").unwrap();

    engine.sync().await?;

    let found = engine.list(ListMode::Matched(info.to_string())).await?;
    if found.len() == 0 {
        println!("no components found");
        return Ok(());
    }

    for c in found.iter() {
        println!("{}: {}", c.id, c.about);
    }
    Ok(())
}
