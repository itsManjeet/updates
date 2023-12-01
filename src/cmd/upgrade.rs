use clap::{arg, Arg, ArgAction, ArgMatches, Command, value_parser};

use ostree::{gio::Cancellable, AsyncProgress};
use swupd::engine::{Engine, Error, UpdateResult};

pub fn cmd() -> Command {
    Command::new("upgrade")
        .about("Upgrade System")
        .long_about("Apply system updates")
        .arg(arg!(<REF> ... "extra extensions to install")
            .value_parser(value_parser!(String))
            .required(false))
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

    let extensions = args.get_many::<String>("REF")
        .into_iter()
        .flatten()
        .map(String::as_str)
        .collect::<Vec<_>>();

    match engine.pull(false, args.get_flag("no-extensions"), None, Some(extensions), Some(&progress), cancellable).await? {
        UpdateResult::NoUpdates => println!("no update available"),
        UpdateResult::UpdatesAvailable(update_info) => {
            engine.deploy(&update_info.refspec, update_info.extensions.iter().map(|s| &**s).collect::<Vec<_>>(), Cancellable::NONE).await?;
        }
    };

    Ok(())
}
