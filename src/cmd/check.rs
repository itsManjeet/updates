use clap::{ArgMatches, Command};

use ostree::{gio::Cancellable, AsyncProgress};
use swupd::engine::{Error, Engine, UpdateResult};


pub fn cmd() -> Command {
    Command::new("check").about("Check for system updates")
}

pub async fn run(_: &ArgMatches, engine: &Engine) -> Result<(), Error> {
    let cancellable = Cancellable::NONE;
    let progress = AsyncProgress::new();
    progress.connect_changed(swupd::progress::update_callback);

    let result = engine.pull(true, false, None, None, Some(&progress), cancellable).await?;
    match result {
        UpdateResult::NoUpdates => println!("System is upto date"),
        UpdateResult::UpdatesAvailable(update_info) => {
            println!("{}", update_info.changelog);
            if update_info.core {
                println!("  core updates: true");
            }
        }
    }

    Ok(())
}
