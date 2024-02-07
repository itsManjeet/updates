use crate::{
    engine::{Engine, RefData},
    Error,
};
use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};
use ostree::gio::Cancellable;
use tracing::info;

pub fn cmd() -> Command {
    Command::new("update")
        .about("Update deployment")
        .arg(
            Arg::new("include")
                .short('i')
                .long("include")
                .help("Include extension in next deployment")
                .action(ArgAction::Append)
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("channel")
                .short('c')
                .long("channel")
                .help("Switch base channel")
                .action(ArgAction::Set)
                .required(false)
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("reset")
                .long("reset")
                .help("Disable all extensions")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("check")
                .long("check")
                .help("Only check for updates don't apply")
                .action(ArgAction::SetTrue),
        )
}

pub async fn run(args: &ArgMatches, engine: &Engine) -> Result<(), Error> {
    let cancellable = Cancellable::NONE;
    let progress = crate::progress::get();

    let include = args
        .get_many::<String>("include")
        .into_iter()
        .flatten()
        .map(|s| s.clone())
        .collect::<Vec<_>>();

    let exclude = args
        .get_many::<String>("exclude")
        .into_iter()
        .flatten()
        .map(|s| s.clone())
        .collect::<Vec<_>>();

    let mut state = engine.state()?.clone();
    if args.get_flag("reset") {
        state.extensions.clear();
    }

    if let Some(channel) = args.get_one::<String>("channel") {
        state.switch_channel(&channel);
    }

    for ext in include.iter() {
        state.add_extension(ext);
    }

    state
        .extensions
        .retain(|s| !exclude.contains(&s.get_data(RefData::Id)));

    let (available, changelog) = engine.check(&state, Some(&progress), cancellable)?;
    if available {
        println!("New updates available");
        println!("{}", changelog);

        if args.get_flag("check") {
            return Ok(());
        }

        info!("Applying updates");
        engine.apply(&state, Some(&progress), cancellable)?;
    }

    Ok(())
}
