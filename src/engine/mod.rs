use std::path::PathBuf;

use ostree::{AsyncProgress, gio::File, Sysroot};
use ostree::gio::Cancellable;
use ostree::glib::{Variant, VariantTy};

use crate::engine::deploy::deploy;
use crate::engine::pull::pull;
use crate::engine::state::State;
use crate::Error;

mod state;
mod pull;
mod deploy;

#[derive(Debug)]
pub struct Engine {
    sysroot: Sysroot,
}

impl Engine {
    pub fn new(root: &PathBuf) -> Result<Engine, Error> {
        let root_file = File::for_path(root);
        let sysroot = Sysroot::new(Some(&root_file));

        sysroot.set_mount_namespace_in_use();
        sysroot.load(Cancellable::NONE)?;

        Ok(Engine { sysroot })
    }

    pub fn lock(&self) -> Result<(), Error> {
        self.sysroot.lock()?;

        Ok(())
    }

    pub fn unlock(&self) {
        self.sysroot.unlock();
    }

    pub fn state(&self) -> Result<State, Error> {
        let osname = match self.sysroot.booted_deployment() {
            Some(deployment) => deployment.osname(),
            None => return Err(Error::NoBootDeployment),
        };

        let merged_deployment = match self.sysroot.merge_deployment(Some(&osname)) {
            Some(deployment) => deployment,
            None => return Err(Error::NoPreviousDeployment),
        };

        State::for_deployment(&self.sysroot.repo(), &merged_deployment)
    }

    pub fn check(
        &self,
        progress: Option<&AsyncProgress>,
        cancellable: Option<&Cancellable>,
    ) -> Result<(bool, String), Error> {
        let current_state = self.state()?;
        let (changed, changelog, _) = pull(&self.sysroot.repo(), &current_state, None, true, progress, cancellable)?;
        Ok((changed, changelog))
    }

    pub fn apply(
        &self,
        progress: Option<&AsyncProgress>,
        cancellable: Option<&Cancellable>,
    ) -> Result<bool, Error> {
        let current_state = self.state()?;
        let (changed, _, state) = pull(&self.sysroot.repo(), &current_state, None, false, progress, cancellable)?;
        if changed {
            deploy(&self.sysroot, &state, cancellable)?;
        }
        Ok(changed)
    }

    pub fn list(
        &self,
        remote: Option<&String>,
        cancellable: Option<&Cancellable>,
    ) -> Result<Vec<String>, Error> {
        let repo = self.sysroot.repo();
        let remote = match remote {
            Some(remote) => remote.clone(),
            None => {
                let remotes = repo.remote_list();
                if remotes.is_empty() {
                    return Err(Error::NoRemoteFound);
                }
                remotes.first().unwrap().to_string()
            }
        };

        let (summary_bytes, _) = repo.remote_fetch_summary(&remote, cancellable)?;
        let summary = Variant::from_bytes_with_type(
            &summary_bytes,
            VariantTy::new("(a(s(taya{sv}))a{sv})").unwrap(),
        );
        let ref_map = summary.child_value(0);
        let mut refs: Vec<String> = Vec::new();
        for r in ref_map.iter() {
            refs.push(r.child_get::<String>(0).clone());
        }

        Ok(refs)
    }
}