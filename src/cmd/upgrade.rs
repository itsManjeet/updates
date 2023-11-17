use std::path::PathBuf;

use clap::{ArgMatches, Command};
use ostree::{
    gio::Cancellable, AsyncProgress, RepoPullFlags, Sysroot, SysrootUpgrader,
    SysrootUpgraderPullFlags,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("GLib Error")]
    GLibError(#[from] ostree::glib::Error),

    #[error("failed to aquire lock")]
    FailedToAquireLock,
}

pub fn cmd() -> Command {
    Command::new("upgrade")
        .about("Upgrade System")
        .long_about("Check and apply system updates")
}

pub async fn run(args: &ArgMatches) -> Result<(), Error> {
    let cancellable = Cancellable::NONE;
    let sysroot_path = args.get_one::<PathBuf>("sysroot").unwrap();
    let sysroot_file = ostree::gio::File::for_uri(&sysroot_path.to_str().unwrap());
    let sysroot = Sysroot::new(Some(&sysroot_file));

    sysroot.load(cancellable)?;

    if !sysroot.try_lock()? {
        return Err(Error::FailedToAquireLock);
    }

    let upgrader = SysrootUpgrader::new(&sysroot, cancellable)?;
    if let Some(origin) = upgrader.dup_origin() {
        ostree::Deployment::origin_remove_transient_state(&origin);
        upgrader.set_origin(Some(&origin), cancellable)?;
    }

    let progress = AsyncProgress::new();

    if !upgrader.pull(
        RepoPullFlags::COMMIT_ONLY,
        SysrootUpgraderPullFlags::NONE,
        Some(&progress),
        cancellable,
    )? {
        progress.finish();

        println!("no updates available");
        return Ok(());
    }

    progress.finish();

    let repo = sysroot.repo();
    let origin = upgrader.origin().unwrap();

    let origin_ref_spec = origin.string("origin", "refspec")?;

    let rev = repo.resolve_rev(&origin_ref_spec.as_str(), false)?.unwrap();

    for deployment in sysroot.deployments() {
        if deployment.csum() == rev {
            println!("Latest revision already deployed; pending reboot");
            return Ok(());
        }
    }

    upgrader.pull(
        RepoPullFlags::NONE,
        SysrootUpgraderPullFlags::NONE,
        Some(&progress),
        cancellable,
    )?;
    progress.finish();

    upgrader.deploy(cancellable)?;

    println!("Upgrade successfull!");
    Ok(())
}
