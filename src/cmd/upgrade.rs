use clap::{ArgMatches, Command};

use ostree::{gio::Cancellable, AsyncProgress};
use updatectl::engine::{self, Engine, UpdateResult};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("GLib Error")]
    GLibError(#[from] ostree::glib::Error),

    #[error("Engine")]
    Engine(#[from] engine::Error),
}

pub fn cmd() -> Command {
    Command::new("upgrade")
        .about("Upgrade System")
        .long_about("Apply system updates")
}

pub async fn run(_: &ArgMatches, engine: &Engine) -> Result<(), Error> {
    let cancellable = Cancellable::NONE;
    let progress = AsyncProgress::new();
    progress.connect_changed(swupd::progress::update_callback);

    engine.load(cancellable)?;

    match engine.update(false, Some(&progress), cancellable).await? {
        UpdateResult::NoUpdates => println!("no update available"),
        UpdateResult::PendingReboot => println!("updated already deployed, waiting for reboot"),
        UpdateResult::UpdatesApplied => println!("updated applied successful!"),
        _ => {}
    }

    Ok(())
}
