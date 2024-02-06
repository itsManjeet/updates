use ostree::gio::Cancellable;
use ostree::glib::VariantDict;
use ostree::prelude::*;
use ostree::{AsyncProgress, Repo, RepoPullFlags};
use tracing::info;

use crate::engine::state::{RefState, State};
use crate::Error;

pub fn pull(
    repo: &Repo,
    state: &State,
    remote: Option<&str>,
    dry_run: bool,
    progress: Option<&AsyncProgress>,
    cancellable: Option<&Cancellable>,
) -> Result<(bool, String, State), Error> {
    let mut refs: Vec<String> = Vec::new();
    let (origin_remote, origin_refspec) = ostree::parse_refspec(&state.core.refspec)?;
    let remote = match remote {
        Some(remote) => remote.to_string(),
        None => match origin_remote {
            Some(remote) => remote.to_string(),
            None => "rlxos".to_string(),
        },
    };
    refs.push(origin_refspec.to_string());

    for ext in state.extensions.iter() {
        let (_, extension_refspec) = ostree::parse_refspec(&ext.refspec)?;
        refs.push(extension_refspec.to_string());
    }

    let options = VariantDict::new(None);
    let mut pull_flags = RepoPullFlags::NONE;
    if dry_run {
        pull_flags |= RepoPullFlags::COMMIT_ONLY;
    }

    options.insert("flags", &(pull_flags.bits() as i32));
    options.insert("refs", &&refs[..]);

    info!("Pulling {:?} from {}", refs, &remote);
    repo.pull_with_options(&remote, &options.to_variant(), progress, cancellable)?;
    info!("Pull success");

    if let Some(progress) = progress {
        progress.finish();
        println!("\n");
    }

    let mut changed = false;
    let mut changelog = String::new();
    let mut changed_core = RefState {
        refspec: state.core.refspec.clone(),
        revision: state.core.revision.clone(),
    };

    let (core_updated, core_revision, core_changelog) =
        get_changelog(&repo, &state.core.refspec, &state.core.revision)?;
    if core_updated {
        changed = true;
        changelog.push_str(&core_changelog);
        changed_core.revision = core_revision;
    }

    let mut changed_extensions: Vec<RefState> = Vec::new();

    for extension in &state.extensions {
        let (extension_updated, extension_revision, extension_changelog) =
            get_changelog(&repo, &extension.refspec, &extension.revision)?;
        if extension_updated {
            changed = true;
            changelog.push_str(format!("\n{}", extension_changelog).as_str());
        }
        changed_extensions.push(RefState {
            refspec: extension.refspec.clone(),
            revision: extension_revision,
        });
    }

    
    Ok((
        changed,
        changelog,
        State {
            core: changed_core,
            merged: state.merged,
            extensions: changed_extensions,
        },
    ))
}

fn get_changelog(
    repo: &Repo,
    refspec: &str,
    old_revision: &str,
) -> Result<(bool, String, String), Error> {
    info!("Getting changelog for {}", refspec);
    let updated_revision = match repo.resolve_rev(refspec, false)? {
        Some(revision) => revision,
        None => return Err(Error::NoRevisionForRefSpec(refspec.into())),
    };
    if updated_revision == old_revision {
        return Ok((false, old_revision.to_string(), "".into()));
    }

    info!("Updated revision {}", updated_revision);

    let commit = repo.load_variant(ostree::ObjectType::Commit, &updated_revision)?;
    let subject = commit
        .child_value(3)
        .get::<String>()
        .unwrap_or_else(|| "".into());
    let body = commit
        .child_value(4)
        .get::<String>()
        .unwrap_or_else(|| "".into());

    Ok((
        true,
        updated_revision.to_string(),
        format!(
            "{}: {}\n{}\nrev: {} -> {}\n",
            refspec,
            subject.trim(),
            body.trim(),
            old_revision,
            updated_revision
        ),
    ))
}
