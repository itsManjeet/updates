use std::env;
use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};

use ostree::{gio::Cancellable, AsyncProgress};
use updatectl::engine::{DeployInfo, Engine, Error, UpdateResult};

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

    let (mut core, mut extensions) = engine.deploy_info()?;
    for inc in include {
        if !contains(&extensions, &inc).0 {
            extensions.push(DeployInfo {
                refspec: inc,
                revision: "".to_string(),
            });
        }
    }

    for exc in exclude {
        let (contain, idx) = contains(&extensions, &exc);
        if contain {
            extensions.remove(idx);
        }
    }

    if args.get_flag("reset") {
        extensions.clear();
    }
    if let Some(base_refspec) = base_refspec {
        core.refspec = base_refspec;
    }

    let remote = args.get_one::<String>("remote");
    match engine.pull(core, extensions, remote, args.get_flag("dry-run"), Some(&progress), cancellable).await? {
        UpdateResult::NoUpdates => println!("\nno update available"),
        UpdateResult::UpdatesAvailable(update_info) => {
            if args.get_flag("dry-run") {
                println!("{}", update_info.changelog);
            } else {
                engine.deploy(&update_info, Cancellable::NONE).await?;
            }
        }
    };

    Ok(())
}

fn contains(extensions: &Vec<DeployInfo>, id: &str) -> (bool, usize) {
    for (idx, extension_info) in extensions.iter().enumerate() {
        if extension_info.refspec == id {
            return (true, idx);
        }
    }
    (false, 0)
}