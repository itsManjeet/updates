use ostree::{
    gio, glib, AsyncProgress, RepoPullFlags, Sysroot, SysrootUpgrader, SysrootUpgraderPullFlags,
};
use std::path::PathBuf;
use thiserror::Error;

pub enum UpdateResult {
    UpdatesAvailable(String),
    UpdatesApplied,
    PendingReboot,
    NoUpdates,
}

pub struct Engine {
    pub sysroot: Sysroot,
}

impl Engine {
    pub fn new(path: Option<&PathBuf>) -> Engine {
        let sysroot: Sysroot;
        if let Some(path) = path {
            let file = gio::File::for_path(path);
            sysroot = Sysroot::new(Some(&file));
        } else {
            sysroot = Sysroot::new_default();
        }

        Engine { sysroot: sysroot }
    }

    pub fn load(
        &self,
        cancellable: Option<&impl glib::IsA<gio::Cancellable>>,
    ) -> Result<(), Error> {
        self.sysroot.set_mount_namespace_in_use();
        self.sysroot.load(cancellable)?;

        if !self.sysroot.try_lock()? {
            return Err(Error::FailedLock);
        }

        self.sysroot.connect_journal_msg(|_, mesg| {
            println!("{}", mesg);
        });
        Ok(())
    }

    pub fn setup_namespace() -> Result<(), Error> {
        match unsafe { syscalls::syscall!(syscalls::Sysno::unshare, 0x00020000) } {
            Err(error) => return Err(Error::FailedSetupNamespace(error)),
            Ok(_) => Ok(()),
        }
    }

    pub async fn update(
        &self,
        dry_run: bool,
        progress: Option<&AsyncProgress>,
        cancellable: Option<&impl glib::IsA<gio::Cancellable>>,
    ) -> Result<UpdateResult, Error> {
        let upgrader = SysrootUpgrader::new(&self.sysroot, cancellable)?;

        if let Some(origin) = upgrader.dup_origin() {
            ostree::Deployment::origin_remove_transient_state(&origin);
            upgrader.set_origin(Some(&origin), cancellable)?;
        }

        let flags = match dry_run {
            true => RepoPullFlags::COMMIT_ONLY,
            false => RepoPullFlags::NONE,
        };

        let changed =
            upgrader.pull(flags, SysrootUpgraderPullFlags::NONE, progress, cancellable)?;
        if let Some(progress) = progress {
            progress.finish();
        }

        if !changed {
            self.sysroot.cleanup(cancellable)?;
            return Ok(UpdateResult::NoUpdates);
        }

        let repo = self.sysroot.repo();
        let origin = upgrader.origin().unwrap();
        let refspec = origin.string("origin", "refspec")?;
        let rev = repo.resolve_rev(&refspec.as_str(), false)?.unwrap();

        for deployment in self.sysroot.deployments() {
            if deployment.csum() == rev {
                return Ok(UpdateResult::PendingReboot);
            }
        }

        if dry_run {
            let commit = repo.load_variant(ostree::ObjectType::Commit, rev.as_str())?;
            let subject = commit.child_get::<String>(3);
            let body = commit.child_get::<String>(4);
            let timestamp = commit.child_get::<u64>(5);

            return Ok(UpdateResult::UpdatesAvailable(format!(
                "{timestamp}:{subject}\n{body}"
            )));
        }

        upgrader.deploy(cancellable)?;

        return Ok(UpdateResult::UpdatesApplied);
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("GLib Error")]
    GLibError(#[from] ostree::glib::Error),

    #[error("failed to setup namespace {0}")]
    FailedSetupNamespace(syscalls::Errno),

    #[error("failed to aquire lock")]
    FailedLock,
}
