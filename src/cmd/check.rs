use clap::{ArgMatches, Command};

use ostree::{gio::Cancellable, AsyncProgress};
use swupd::engine::{self, Engine, UpdateResult};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("GLib Error")]
    GLibError(#[from] ostree::glib::Error),

    #[error("Engine")]
    Engine(#[from] engine::Error),
}

pub fn cmd() -> Command {
    Command::new("check").about("Check for system updates")
}

pub async fn run(_: &ArgMatches, engine: &Engine) -> Result<(), Error> {
    let cancellable = Cancellable::NONE;
    let progress = AsyncProgress::new();
    progress.connect_changed(swupd::progress::update_callback);

    engine.load(cancellable)?;

    match engine.update(true, Some(&progress), cancellable).await? {
        UpdateResult::NoUpdates => println!("no update available"),
        UpdateResult::PendingReboot => println!("updated already deployed, waiting for reboot"),
        UpdateResult::UpdatesApplied => println!("updated applied successful!"),
        UpdateResult::UpdatesAvailable(changelog) => println!("updates available\n{}", changelog),
    }

    Ok(())
}
