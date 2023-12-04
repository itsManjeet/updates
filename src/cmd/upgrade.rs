use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};

use ostree::{gio::Cancellable, AsyncProgress};
use swupd::engine::{Engine, Error, UpdateResult};

pub fn cmd() -> Command {
    Command::new("upgrade")
        .about("Upgrade System")
        .long_about("Apply system updates")
        .arg(Arg::new("add")
            .short('a')
            .long("add")
            .help("Add extension in next deployment")
            .action(ArgAction::Set)
            .value_parser(value_parser!(String)))
        .arg(Arg::new("remove")
            .short('r')
            .long("remove")
            .help("Remove extension in next deployment (skip if not already installed)")
            .action(ArgAction::Set)
            .value_parser(value_parser!(String)))

        .arg(Arg::new("no-extensions")
            .short('N')
            .long("no-extensions")
            .help("Disable all extensions")
            .action(ArgAction::SetTrue))
}

pub async fn run(args: &ArgMatches, engine: &Engine) -> Result<(), Error> {
    let cancellable = Cancellable::NONE;
    let progress = AsyncProgress::new();
    progress.connect_changed(swupd::progress::update_callback);

    let add_extensions = args.get_many::<String>("add")
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let remove_extensions = args.get_many::<String>("remove")
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    match engine.pull(false, args.get_flag("no-extensions"), None, Some(add_extensions), Some(remove_extensions), Some(&progress), cancellable).await? {
        UpdateResult::NoUpdates => println!("no update available"),
        UpdateResult::UpdatesAvailable(update_info) => {
            engine.deploy(&update_info, Cancellable::NONE).await?;
        }
    };

    Ok(())
}
