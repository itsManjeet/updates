use std::collections::HashMap;
use std::{env, ptr};
use ostree::{AsyncProgress, Deployment, ffi, gio, glib, MutableTree, ObjectType, Repo, RepoFile, RepoPullFlags, Sysroot, SysrootSimpleWriteDeploymentFlags};
use ostree::gio::Cancellable;
use ostree::glib::{Cast, IsA, KeyFile, ToVariant, VariantDict, VariantTy};
use ostree::glib::translate::{from_glib_full, ToGlibPtr};
use thiserror::Error;


pub struct Engine {
    pub sysroot: Sysroot,
    osname: String,
    deployment: Deployment,
    origin: KeyFile,
}

impl Engine {
    pub fn new(sysroot: Sysroot, osname: Option<&str>) -> Result<Engine, Error> {
        let osname = match osname {
            Some(osname) => osname.to_string(),
            None => {
                let osname = match sysroot.booted_deployment() {
                    Some(deployment) => deployment.osname().to_string(),
                    None => return Err(Error::NoBootDeployment),
                };
                osname
            }
        };
        let deployment = match sysroot.merge_deployment(Some(&osname)) {
            Some(deployment) => deployment,
            None => return Err(Error::NoPreviousDeployment),
        };

        let origin = match deployment.origin() {
            Some(origin) => origin,
            None => return Err(Error::NoOriginForDeployment(deployment.csum().to_string(), deployment.deployserial())),
        };

        Ok(Engine { sysroot, osname, deployment, origin })
    }

    pub async fn deploy(&self, base_refspec: &str, extensions: Vec<&str>, cancellable: Option<&Cancellable>) -> Result<(), Error> {
        let repo = self.sysroot.repo();
        repo.is_writable()?;

        let refspec = self.origin.string("origin", "refspec")?;
        let rev: String;
        let origin: KeyFile;
        if !(refspec == base_refspec && extensions.is_empty()) {
            let options = VariantDict::new(None);
            options.insert("rlxos.merged", &true);

            let base_rev = repo.resolve_rev(base_refspec, false)?.unwrap().to_string();
            options.insert("rlxos.base-checksum", &base_rev);

            repo.prepare_transaction(cancellable)?;
            let mutable_tree = MutableTree::from_commit(&repo, &base_rev)?;

            let mut ext_list: Vec<&str> = Vec::new();
            for ext in extensions {
                ext_list.push(ext);
                let ext_rev = repo.resolve_rev(base_refspec, false)?.unwrap();
                options.insert(&format!("rlxos.ext-checksum-{}", ext.replace("/", "-")), &ext_rev.as_str());
                let (object_to_commit, _) = repo.read_commit(ext, cancellable)?;
                repo.write_directory_to_mtree(&object_to_commit, &mutable_tree, None, cancellable)?;
            }
            options.insert("rlxos.ext-list", &&ext_list[..]);


            let root = repo.write_mtree(&mutable_tree, cancellable)?;
            let boot_meta = VariantDict::new(None);
            commit_metadata_for_bootable(&root, &boot_meta, cancellable)?;

            let root = root.downcast_ref::<RepoFile>().unwrap();
            let commit_checksum = repo.write_commit(None, None, None, Some(&options.to_variant()), &root, cancellable)?;

            let deployment_refspec = format!("{}/os/local", env::consts::ARCH);
            repo.transaction_set_ref(None, &deployment_refspec, Some(&commit_checksum));
            let _stats = repo.commit_transaction(cancellable)?;

            rev = repo.resolve_rev(&deployment_refspec, false)?.unwrap().to_string();
            origin = self.sysroot.origin_new_from_refspec(&deployment_refspec);
            println!("refspec: {}", &deployment_refspec);
        } else {
            rev = repo.resolve_rev(&refspec, false)?.unwrap().to_string();
            origin = self.sysroot.origin_new_from_refspec(&refspec);
            println!("refspec: {}", &refspec);
        }

        let opts = ostree::SysrootDeployTreeOpts {
            ..Default::default()
        };

        let deployment = self.sysroot.deploy_tree_with_options(Some(&self.osname), &rev, Some(&origin), Some(&self.deployment), Some(&opts), cancellable)?;
        let flags = SysrootSimpleWriteDeploymentFlags::NO_CLEAN;
        self.sysroot.simple_write_deployment(Some(&self.osname), &deployment, Some(&self.deployment), flags, cancellable)?;

        self.sysroot.cleanup(cancellable)?;
        Ok(())
    }


    pub async fn pull(&self, dry_run: bool, disable_ext: bool, remote: Option<&str>, extra_ext: Option<Vec<&str>>, progress: Option<&AsyncProgress>, cancellable: Option<&Cancellable>) -> Result<UpdateResult, Error> {
        let repo = self.sysroot.repo();

        let remote = match remote {
            Some(remote) => remote.to_string(),
            None => self.osname.clone()
        };


        let ((base_refspec, base_timestamp), mut extensions) = parse_deployment(&repo, &self.deployment)?;
        if disable_ext {
            extensions.clear();
        }

        if let Some(extra_ext) = extra_ext {
            for ext in extra_ext {
                extensions.insert(ext.to_string(), 0);
            }
        }

        let mut refs: Vec<&str> = Vec::new();
        refs.push(base_refspec.as_str());

        for (r, _) in extensions.iter() {
            refs.push(r.as_str());
        }

        let options = VariantDict::new(None);

        let mut pull_flags = RepoPullFlags::NONE;
        if dry_run { pull_flags |= RepoPullFlags::COMMIT_ONLY; }

        options.insert("flags", &(pull_flags.bits() as i32));
        options.insert("refs", &&refs[..]);

        repo.pull_with_options(&remote, &options.to_variant(), progress, cancellable)?;
        if let Some(progress) = progress {
            progress.finish();
        }

        let base_new_timestamp = get_timestamp_of_ref(&repo, &base_refspec)?;
        let mut changelog = String::new();
        let core_updated = match base_new_timestamp == base_timestamp {
            true => {
                changelog.push_str(&gen_changelog(&repo, &base_refspec)?);
                true
            }
            false => false,
        };

        let mut updated_ext: Vec<String> = Vec::new();
        for (ext_refspec, timestamp) in extensions.iter() {
            let new_timestamp = get_timestamp_of_ref(&repo, &ext_refspec)?;
            if &new_timestamp != timestamp {
                changelog.push_str(&gen_changelog(&repo, &ext_refspec)?);
                updated_ext.push(ext_refspec.clone());
            }
        }

        if !core_updated && updated_ext.is_empty() {
            return Ok(UpdateResult::NoUpdates);
        }

        let update_info = UpdateInfo {
            refspec: base_refspec,
            core: core_updated,
            extensions: updated_ext,
            changelog,
        };


        Ok(UpdateResult::UpdatesAvailable(update_info))
    }
}

pub struct UpdateInfo {
    pub refspec: String,
    pub core: bool,
    pub extensions: Vec<String>,
    pub changelog: String,
}


pub enum UpdateResult {
    NoUpdates,
    UpdatesAvailable(UpdateInfo),
}


pub fn parse_deployment(repo: &Repo, deployment: &Deployment) -> Result<((String, u64), HashMap<String, u64>), Error> {
    let origin = deployment.origin().unwrap();
    let refspec = origin.string("origin", "refspec")?;
    let channel = match origin.string("origin", "channel") {
        Ok(channel) => channel.to_string(),
        Err(_) => String::from("stable"),
    };

    let mut timestamps: HashMap<String, u64> = HashMap::new();
    let rev = match repo.resolve_rev(&refspec, false)? {
        Some(rev) => rev,
        None => return Err(Error::NoRevisionForRefSpec(refspec.to_string())),
    };

    let commit = repo.load_variant(ObjectType::Commit, rev.as_str())?;
    let commit_metadata = VariantDict::new(Some(&commit.child_value(0)));

    let merged = match commit_metadata.lookup_value("rlxos.merged", Some(&VariantTy::BOOLEAN)) {
        Some(merged) => merged.get::<bool>().unwrap(),
        None => false,
    };

    if !merged {
        let timestamp = ostree::commit_get_timestamp(&commit);
        return Ok(((refspec.to_string(), timestamp), timestamps));
    }

    let base_checksum = match commit_metadata.lookup_value("rlxos.base-checksum", Some(&VariantTy::STRING)) {
        Some(base_checksum) => base_checksum.get::<String>().unwrap(),
        None => return Err(Error::NoBaseCheckSum),
    };
    let base_checksum_commit = repo.load_variant(ObjectType::Commit, base_checksum.as_str())?;
    let base_checksum_timestamp = ostree::commit_get_timestamp(&base_checksum_commit);


    let extensions_list = match commit_metadata.lookup_value("rlxos.ext-list", Some(&VariantTy::STRING_ARRAY)) {
        Some(extension_list) => Some(extension_list.get::<Vec<String>>().unwrap()),
        None => None,
    };

    if let Some(ext_list) = extensions_list {
        println!("ext list: {:?}", &ext_list);
        for ext in ext_list.iter() {
            let ext_checksum = match commit_metadata.lookup_value(&format!("rlxos.ext-checksum-{}", &ext.replace("/", "-")), Some(&VariantTy::STRING)) {
                Some(ext_checksum) => ext_checksum.get::<String>().unwrap(),
                None => return Err(Error::NoExtCheckSum(ext.clone())),
            };
            let ext_commit = repo.load_variant(ObjectType::Commit, ext_checksum.as_str())?;
            let ext_timestamp = ostree::commit_get_timestamp(&ext_commit);
            timestamps.insert(ext.clone(), ext_timestamp);
        }
    }


    Ok(((format!("{}/os/{}", env::consts::ARCH, channel), base_checksum_timestamp), timestamps))
}


pub fn get_timestamp_of_ref(repo: &Repo, refspec: &str) -> Result<u64, Error> {
    let rev = match repo.resolve_rev(refspec, false)? {
        Some(rev) => rev,
        None => return Err(Error::NoRevisionForRefSpec(refspec.to_string())),
    };

    let commit = repo.load_variant(ObjectType::Commit, rev.as_str())?;
    Ok(ostree::commit_get_timestamp(&commit))
}

pub fn gen_changelog(repo: &Repo, refspec: &str) -> Result<String, Error> {
    let rev = match repo.resolve_rev(refspec, false)? {
        Some(rev) => rev,
        None => return Err(Error::NoRevisionForRefSpec(refspec.to_string())),
    };

    let commit = repo.load_variant(ObjectType::Commit, rev.as_str())?;
    let subject = match commit.child_value(3).get::<String>() {
        Some(subject) => subject,
        None => String::from(""),
    };

    let body = match commit.child_value(4).get::<String>() {
        Some(body) => body,
        None => String::from(""),
    };

    Ok(format!("{}: {}\n{}", refspec, subject, body))
}

pub fn commit_metadata_for_bootable(root: &impl IsA<gio::File>, options: &VariantDict, cancellable: Option<&impl IsA<Cancellable>>) -> Result<(), glib::Error> {
    unsafe {
        let mut error = ptr::null_mut();
        let is_ok = ffi::ostree_commit_metadata_for_bootable(root.as_ref().to_glib_none().0, options.to_glib_none().0, cancellable.map(|p| p.as_ref()).to_glib_none().0, &mut error);
        assert_eq!(is_ok == glib::ffi::GFALSE, !error.is_null());

        if error.is_null() { Ok(()) } else { Err(from_glib_full(error)) }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("glib")]
    GLib(#[from] glib::Error),

    #[error("no boot deployment")]
    NoBootDeployment,

    #[error("no previous deployment")]
    NoPreviousDeployment,

    #[error("no origin known for deployment {0}.{1}")]
    NoOriginForDeployment(String, i32),

    #[error("no revision for refspec {0}")]
    NoRevisionForRefSpec(String),

    #[error("no base checksum")]
    NoBaseCheckSum,

    #[error("no extension checksum {0}")]
    NoExtCheckSum(String),

    #[error("failed to prepare transaction")]
    FailedPrepareTransaction,
}