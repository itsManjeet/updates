use chrono::prelude::DateTime;
use chrono::Utc;
use ostree::gio::Cancellable;
use ostree::glib::translate::{from_glib_full, ToGlibPtr};
use ostree::glib::{Cast, GString, IsA, KeyFile, ToVariant, Variant, VariantDict, VariantTy};
use ostree::{
    ffi, gio, glib, AsyncProgress, Deployment, MutableTree, ObjectType, Repo, RepoFile,
    RepoPullFlags, Sysroot, SysrootSimpleWriteDeploymentFlags,
};
use std::time::{Duration, UNIX_EPOCH};
use std::{env, ptr};
use thiserror::Error;

const STRING_LIST_SEP: &str = ";";

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
            None => {
                return Err(Error::NoOriginForDeployment(
                    deployment.csum().to_string(),
                    deployment.deployserial(),
                ));
            }
        };

        Ok(Engine {
            sysroot,
            osname,
            deployment,
            origin,
        })
    }

    pub async fn deploy(
        &self,
        update_info: &UpdateInfo,
        cancellable: Option<&Cancellable>,
    ) -> Result<(), Error> {
        let repo = self.sysroot.repo();
        repo.is_writable()?;

        let refspec = self.origin.string("origin", "refspec")?;
        let rev: String;
        let origin: KeyFile;

        if update_info.merged {
            let options = VariantDict::new(None);
            options.insert(get_revision_key("core").as_str(), &update_info.core.revision);

            repo.prepare_transaction(cancellable)?;
            let mutable_tree = MutableTree::from_commit(&repo, &update_info.core.revision)?;

            let mut extensions: Vec<&str> = Vec::new();
            for extension_info in &update_info.extensions {
                extensions.push(extension_info.refspec.as_str());
                options.insert(get_revision_key(get_extension_id(&extension_info.refspec).as_str()).as_str(),
                               &extension_info.revision);
                let (object_to_commit, _) = repo.read_commit(&extension_info.refspec, cancellable)?;
                repo.write_directory_to_mtree(&object_to_commit, &mutable_tree, None, cancellable)?;
            }

            let root = repo.write_mtree(&mutable_tree, cancellable)?;
            let boot_meta = VariantDict::new(None);
            commit_metadata_for_bootable(&root, &boot_meta, cancellable)?;

            let root = root.downcast_ref::<RepoFile>().unwrap();
            let commit_checksum = repo.write_commit(
                None,
                None,
                None,
                Some(&options.to_variant()),
                &root,
                cancellable,
            )?;

            let deployment_refspec = format!("{}/os/local", env::consts::ARCH);
            repo.transaction_set_ref(None, &deployment_refspec, Some(&commit_checksum));
            let _stats = repo.commit_transaction(cancellable)?;

            rev = repo
                .resolve_rev(&deployment_refspec, false)?
                .unwrap()
                .to_string();
            origin = self.sysroot.origin_new_from_refspec(&deployment_refspec);
            origin.set_string("rlxos", "extensions", extensions.join(";").as_str());
            origin.set_string("rlxos", "refspec", &update_info.core.refspec);

            origin.set_boolean("rlxos", "merged", true);
        } else {
            rev = repo.resolve_rev(&refspec, false)?.unwrap().to_string();
            origin = self.sysroot.origin_new_from_refspec(&refspec);
        }

        let opts = ostree::SysrootDeployTreeOpts {
            ..Default::default()
        };

        let deployment = self.sysroot.deploy_tree_with_options(
            Some(&self.osname),
            &rev,
            Some(&origin),
            Some(&self.deployment),
            Some(&opts),
            cancellable,
        )?;
        let flags = SysrootSimpleWriteDeploymentFlags::NO_CLEAN;
        self.sysroot.simple_write_deployment(
            Some(&self.osname),
            &deployment,
            Some(&self.deployment),
            flags,
            cancellable,
        )?;

        self.sysroot.cleanup(cancellable)?;
        Ok(())
    }

    pub fn deploy_info(&self) -> Result<(DeployInfo, Vec<DeployInfo>), Error> {
        let repo = self.sysroot.repo();
        parse_deployment(&repo, &self.deployment)
    }

    pub async fn pull(
        &self,
        core: DeployInfo,
        extensions: Vec<DeployInfo>,
        remote: Option<&String>,
        dry_run: bool,
        progress: Option<&AsyncProgress>,
        cancellable: Option<&Cancellable>,
    ) -> Result<UpdateResult, Error> {
        let repo = self.sysroot.repo();

        let mut refs: Vec<String> = Vec::new();
        let (base_remote, base_refspec) = ostree::parse_refspec(&core.refspec)?;
        refs.push(base_refspec.to_string());

        let remote = match remote {
            Some(remote) => remote.to_string(),
            None => match base_remote {
                Some(remote) => remote.to_string(),
                None => {
                    let remote_list = repo.remote_list();
                    if remote_list.is_empty() {
                        return Err(Error::NoRemoteFound);
                    }
                    remote_list.first().unwrap().to_string()
                }
            },
        };

        for info in extensions.iter() {
            let (_, ext_refspec) = ostree::parse_refspec(&info.refspec)?;
            refs.push(ext_refspec.to_string());
        }

        let options = VariantDict::new(None);

        let mut pull_flags = RepoPullFlags::NONE;
        if dry_run {
            pull_flags |= RepoPullFlags::COMMIT_ONLY;
        }

        println!("checking refs: {}", refs.join(" "));

        options.insert("flags", &(pull_flags.bits() as i32));
        options.insert("refs", &&refs[..]);

        repo.pull_with_options(&remote, &options.to_variant(), progress, cancellable)?;
        if let Some(progress) = progress {
            progress.finish();
        }
        println!("\n");

        let (base_new_rev, _) = get_rev_timestamp_of_ref(&repo, &core.refspec)?;
        let mut changelog = String::new();

        let mut updates_available = match core.revision != base_new_rev {
            true => {
                changelog.push_str(&gen_changelog(
                    &repo,
                    &core.refspec,
                    &core.revision,
                )?);
                true
            }
            false => false,
        };

        let mut updated_extensions: Vec<DeployInfo> = Vec::new();
        for extension_info in extensions.iter() {
            let (new_revision, _) = get_rev_timestamp_of_ref(&repo, &extension_info.refspec)?;
            if &extension_info.revision != &new_revision {
                changelog.push_str(&gen_changelog(
                    &repo,
                    &extension_info.refspec,
                    &extension_info.revision,
                )?);
                updates_available = true;
            }
            updated_extensions.push(DeployInfo {
                refspec: extension_info.refspec.clone(),
                revision: new_revision,
            });
        }

        if !updates_available {
            return Ok(UpdateResult::NoUpdates);
        }

        let update_info = UpdateInfo {
            core: DeployInfo {
                refspec: core.refspec,
                revision: base_new_rev,
            },
            merged: !updated_extensions.is_empty(),
            extensions: updated_extensions,
            changelog,
        };

        Ok(UpdateResult::UpdatesAvailable(update_info))
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

#[derive(Debug)]
pub struct UpdateInfo {
    pub core: DeployInfo,
    pub extensions: Vec<DeployInfo>,
    pub merged: bool,
    pub changelog: String,
}

pub enum UpdateResult {
    NoUpdates,
    UpdatesAvailable(UpdateInfo),
}

#[derive(Debug)]
pub struct DeployInfo {
    pub refspec: String,
    pub revision: String,
}

pub fn parse_deployment(
    repo: &Repo,
    deployment: &Deployment,
) -> Result<(DeployInfo, Vec<DeployInfo>), Error> {
    // Get base reference information
    let origin = deployment.origin().unwrap();
    let origin_refspec = origin.string("origin", "refspec")?;
    let merged = origin.boolean("rlxos", "merged").unwrap_or_else(|_| false);

    let revision = deployment.csum().to_string();
    let commit = repo.load_variant(ObjectType::Commit, &revision)?;
    let commit_metadata = VariantDict::new(Some(&commit.child_value(0)));

    if !merged {
        return Ok((
            DeployInfo {
                refspec: origin_refspec.to_string(),
                revision,
            },
            Vec::new(),
        ));
    }

    // If anything is wrong, goto stable
    let refspec = origin.string("rlxos", "refspec")?.to_string();

    // Parse base reference of merge deployment
    let revision =
        match commit_metadata.lookup_value(get_revision_key("core").as_str(), Some(&VariantTy::STRING)) {
            Some(revision) => revision.get::<String>().unwrap(),
            None => return Err(Error::NoBaseCheckSum),
        };

    let extensions_refspec: Vec<String> = origin.string("rlxos", "extensions").unwrap_or_else(|_| GString::from(""))
        .to_string()
        .split(STRING_LIST_SEP)
        .map(|s| s.to_string())
        .collect();

    let mut extensions: Vec<DeployInfo> = Vec::new();
    for extension_refspec in extensions_refspec.iter() {
        let extension_revision = match commit_metadata.lookup_value(
            get_revision_key(&get_extension_id(extension_refspec)).as_str(),
            Some(&VariantTy::STRING),
        ) {
            Some(revision) => revision.get::<String>().unwrap(),
            None => return Err(Error::NoExtCheckSum(extension_refspec.clone())),
        };

        extensions.push(DeployInfo {
            refspec: extension_refspec.clone(),
            revision: extension_revision,
        });
    }

    Ok((
        DeployInfo {
            refspec,
            revision,
        },
        extensions,
    ))
}

pub fn get_rev_timestamp_of_ref(repo: &Repo, refspec: &str) -> Result<(String, u64), Error> {
    let rev = match repo.resolve_rev(refspec, false)? {
        Some(rev) => rev,
        None => return Err(Error::NoRevisionForRefSpec(refspec.to_string())),
    };

    let commit = repo.load_variant(ObjectType::Commit, rev.as_str())?;
    Ok((rev.to_string(), ostree::commit_get_timestamp(&commit)))
}

pub fn gen_changelog(repo: &Repo, refspec: &str, old_rev: &str) -> Result<String, Error> {
    let rev = match repo.resolve_rev(refspec, false)? {
        Some(rev) => rev,
        None => return Err(Error::NoRevisionForRefSpec(refspec.to_string())),
    };

    let commit = repo.load_variant(ObjectType::Commit, rev.as_str())?;
    let subject = commit.child_value(3).get::<String>().unwrap_or_else(|| String::from(""));
    let body = commit.child_value(4).get::<String>().unwrap_or_else(|| String::from(""));

    Ok(format!(
        "{}: {}\n{}\nrev: {} -> {}\n",
        refspec, subject, body, old_rev, &rev
    ))
}

pub fn commit_metadata_for_bootable(
    root: &impl IsA<gio::File>,
    options: &VariantDict,
    cancellable: Option<&impl IsA<Cancellable>>,
) -> Result<(), glib::Error> {
    unsafe {
        let mut error = ptr::null_mut();
        let is_ok = ffi::ostree_commit_metadata_for_bootable(
            root.as_ref().to_glib_none().0,
            options.to_glib_none().0,
            cancellable.map(|p| p.as_ref()).to_glib_none().0,
            &mut error,
        );
        assert_eq!(is_ok == glib::ffi::GFALSE, !error.is_null());

        if error.is_null() {
            Ok(())
        } else {
            Err(from_glib_full(error))
        }
    }
}

pub fn format_timestamp(timestamp: u64) -> String {
    let d = UNIX_EPOCH + Duration::from_secs(timestamp);
    let datetime = DateTime::<Utc>::from(d);
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn get_revision_key(id: &str) -> String {
    format!("rlxos.revision.{id}")
}

fn get_extension_id(refspec: &str) -> String {
    refspec.to_string().split("/").map(|s| s.to_string()).collect::<Vec<String>>()[2].clone()
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

    #[error("permission error {0}")]
    PermissionError(String),

    #[error("failed to lock sysroot")]
    FailedTryLock,

    #[error("failed to setup namespace {0}")]
    FailedSetupNamespace(syscalls::Errno),

    #[error("no remote found")]
    NoRemoteFound,
}
