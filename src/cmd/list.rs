use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};

use ostree::{gio::Cancellable, AsyncProgress};
use updatectl::engine::{Engine, Error};

pub fn cmd() -> Command {
    Command::new("list")
        .about("List available extensions")
        .long_about("Print available extensions from remote server")
        .arg(Arg::new("remote")
            .short('r')
            .long("remove")
            .help("Specify remote server name or url")
            .action(ArgAction::Set)
            .value_parser(value_parser!(String)))
        .arg(Arg::new("all")
            .short('a')
            .long("all")
            .help("List all available ostree references")
            .action(ArgAction::SetTrue))
}

pub async fn run(args: &ArgMatches, engine: &Engine) -> Result<(), Error> {
    let cancellable = Cancellable::NONE;
    let progress = AsyncProgress::new();
    progress.connect_changed(updatectl::progress::update_callback);

    let refs = engine.list(args.get_one::<String>("remote"), cancellable)?;
    if refs.is_empty() {
        println!("no extensions found");
    } else {
        for (i, r) in refs.iter().enumerate() {
            if r.starts_with("x86_64/extension/") || args.get_flag("all") {
                println!("{}. {}", i + 1, r);
            }
        }
    }

    Ok(())
}
