use std::env;
use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};

use ostree::{gio::Cancellable, AsyncProgress};
use updatectl::engine::{Engine, Error, PullOpts, UpdateResult};

pub fn cmd() -> Command {
    Command::new("update")
        .about("Update deployment")
        .arg(Arg::new("include")
            .short('i')
            .long("include")
            .help("Include extension in next deployment")
            .action(ArgAction::Append)
            .value_parser(value_parser!(String)))
        .arg(Arg::new("exclude")
            .short('e')
            .long("exclude")
            .help("Exclude extension in next deployment (skip if not already installed)")
            .action(ArgAction::Append)
            .value_parser(value_parser!(String)))
        .arg(Arg::new("channel")
            .short('c')
            .long("channel")
            .help("Switch base channel")
            .action(ArgAction::Set)
            .required(false)
            .value_parser(value_parser!(String)))
        .arg(Arg::new("reset")
            .long("reset")
            .help("Disable all extensions")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("dry-run")
            .long("dry-run")
            .help("Dry run")
            .action(ArgAction::SetTrue))
}

pub async fn run(args: &ArgMatches, engine: &Engine) -> Result<(), Error> {
    let cancellable = Cancellable::NONE;
    let progress = AsyncProgress::new();
    progress.connect_changed(updatectl::progress::update_callback);

    let include = args.get_many::<String>("include")
        .into_iter()
        .flatten()
        .map(|s| s.clone())
        .collect::<Vec<_>>();

    let exclude = args.get_many::<String>("exclude")
        .into_iter()
        .flatten()
        .map(|s| s.clone())
        .collect::<Vec<_>>();

    let base_refspec = match args.get_one::<String>("channel") {
        Some(channel) => {
            let channel = if channel.starts_with(&format!("{}/os/", env::consts::ARCH)) {
                Some(channel.clone())
            } else {
                Some(format!("{}/os/{}", env::consts::ARCH, channel))
            };

            channel
        }
        None => None,
    };

    let pull_opts = PullOpts {
        dry_run: args.get_flag("dry-run"),
        reset: args.get_flag("reset"),
        remote: args.get_one::<String>("remote").and_then(|s| { Some(s.clone()) }),

        include,
        exclude,

        base_refspec,
    };

    match engine.pull(&pull_opts, Some(&progress), cancellable).await? {
        UpdateResult::NoUpdates => println!("\nno update available"),
        UpdateResult::UpdatesAvailable(update_info) => {
            if pull_opts.dry_run {
                println!("{}", update_info.changelog);
            } else {
                engine.deploy(&update_info, Cancellable::NONE).await?;
            }
        }
    };

    Ok(())
}
