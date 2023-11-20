use clap::{Arg, ArgAction, ArgMatches, Command};
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
        .arg(
            Arg::new("check")
                .short('c')
                .long("check")
                .help("Check for updates only")
                .action(ArgAction::SetTrue),
        )
}

pub async fn run(args: &ArgMatches) -> Result<(), Error> {
    let cancellable = Cancellable::NONE;
    let sysroot = Sysroot::new_default();

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

    if args.get_flag("check") {
        let commit_info = repo.load_variant(ostree::ObjectType::Commit, rev.as_str())?;
        let subject = commit_info.child_get::<String>(3);
        let body = commit_info.child_get::<String>(4);
        let timestamp = commit_info.child_get::<u64>(5);

        println!("{timestamp}:{subject}\n{body}");
    } else {
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
    }

    Ok(())
}
