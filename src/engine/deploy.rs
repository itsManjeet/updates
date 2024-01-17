use std::{env, ptr};

use ostree::{ffi, gio, glib, MutableTree, RepoFile, Sysroot, SysrootSimpleWriteDeploymentFlags};
use ostree::gio::Cancellable;
use ostree::glib::{Cast, IsA, KeyFile, ToVariant, VariantDict};
use ostree::glib::translate::{from_glib_full, ToGlibPtr};

use crate::engine::state::State;
use crate::Error;

pub fn deploy(
    sysroot: &Sysroot,
    state: &State,
    cancellable: Option<&Cancellable>,
) -> Result<(), Error> {
    let osname = match sysroot.booted_deployment() {
        Some(deployment) => deployment.osname(),
        None => "rlxos".into(),
    };
    let deployment = sysroot.merge_deployment(Some(&osname)).unwrap();
    let repo = sysroot.repo();
    repo.is_writable()?;

    let revision: String;
    let origin: KeyFile;

    if state.merged {
        let (options, extensions) = state.options();

        repo.prepare_transaction(cancellable)?;
        let mutable_tree = MutableTree::from_commit(&repo, &state.core.revision)?;

        if let Some(extensions) = &state.extensions {
            for extension in extensions {
                let (object_to_commit, _) = repo.read_commit(&extension.refspec, cancellable)?;
                repo.write_directory_to_mtree(&object_to_commit, &mutable_tree, None, cancellable)?;
            }
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

        revision = repo.resolve_rev(&deployment_refspec, false)?.unwrap().to_string();

        origin = sysroot.origin_new_from_refspec(&deployment_refspec);
        origin.set_string("rlxos", "extensions", &extensions);
        origin.set_boolean("rlxos", "merged", true);
        origin.set_string("rlxos", "channel", &state.core.refspec.to_string().split("/").map(|s| s.to_string()).collect::<Vec<String>>().last().unwrap())
    } else {
        revision = state.core.revision.clone();
        origin = sysroot.origin_new_from_refspec(&state.core.refspec);
        origin.set_boolean("rlxos", "merged", false);
    }

    let opts = ostree::SysrootDeployTreeOpts {
        ..Default::default()
    };

    let new_deployment = sysroot.deploy_tree_with_options(
        Some(&osname),
        &revision,
        Some(&origin),
        Some(&deployment),
        Some(&opts),
        cancellable,
    )?;
    let flags = SysrootSimpleWriteDeploymentFlags::NO_CLEAN;
    sysroot.simple_write_deployment(
        Some(&osname),
        &new_deployment,
        Some(&deployment),
        flags,
        cancellable,
    )?;

    sysroot.cleanup(cancellable)?;
    Ok(())
}


fn commit_metadata_for_bootable(
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