use clap::{ArgMatches, Command};
use ostree::{COMMIT_META_KEY_SOURCE_TITLE, COMMIT_META_KEY_VERSION, DeploymentUnlockedState};
use ostree::glib::{VariantDict, VariantTy};
use swupd::engine;
use swupd::engine::{Engine, Error};

pub fn cmd() -> Command {
    Command::new("status")
        .about("Show deployment status")
        .long_about("Check and apply system updates")
}

pub async fn run(_: &ArgMatches, engine: &Engine) -> Result<(), Error> {
    let deployments = engine.sysroot.deployments();
    if deployments.len() == 0 {
        println!("no deployment found!!");
        return Ok(());
    }

    let booted_deployment = engine.sysroot.booted_deployment();
    let (pending_deployment, rollback_deployment) = match booted_deployment {
        Some(_) => engine.sysroot.query_deployments_for(None),
        None => (None, None)
    };

    for deployment in deployments {
        let mut status: String = String::new();
        let mut is_booted = String::new();
        if deployment.is_staged() {
            status = String::from("staged");
        } else if let Some(pending_deployment) = pending_deployment.clone() {
            if deployment == pending_deployment {
                status = String::from("pending");
            }
        } else if let Some(rollback) = rollback_deployment.clone() {
            if deployment == rollback {
                status = String::from("rollback");
            }
        }

        if let Some(booted) = booted_deployment.clone() {
            if booted == deployment {
                is_booted = String::from("(active)");
            }
        }
        println!(
            "{} {}.{} {}",
            deployment.osname(),
            truncate(deployment.csum().as_str(), 6),
            deployment.deployserial(),
            is_booted
        );
        if !status.is_empty() {
            println!("  status: {status}");
        }
        println!("  ref: {}.{}", deployment.csum(), deployment.deployserial());

        let repo = &engine.sysroot.repo();
        let ((base_refspec, (base_rev, base_timestamp)), extensions) = engine::parse_deployment(repo, &deployment)?;

        println!("  base_refspec: {}.{}:{}", base_refspec, base_rev, base_timestamp);
        println!("  extensions: {}", &extensions.len());
        for (i, (ext, (rev, timestamp))) in extensions.iter().enumerate() {
            println!("    {}. {}.{}:{}", i + 1, ext, rev, timestamp);
        }

        match repo.load_variant(ostree::ObjectType::Commit, deployment.csum().as_str()) {
            Err(_) => {}
            Ok(commit) => {
                let commit_dict = VariantDict::new(Some(&commit.child_value(0)));
                if let Some(version) = commit_dict.lookup_value(
                    COMMIT_META_KEY_VERSION.to_string().as_str(),
                    Some(VariantTy::STRING),
                ) {
                    println!("\tversion: {}", version.get::<String>().unwrap());
                }

                if let Some(ext_list) = commit_dict.lookup_value("rlxos.ext-list", Some(VariantTy::STRING_ARRAY)) {
                    println!("\textensions list: {:?}", ext_list.get::<Vec<String>>().unwrap());
                }
                if let Some(source_title) = commit_dict.lookup_value(
                    COMMIT_META_KEY_SOURCE_TITLE.to_string().as_str(),
                    Some(VariantTy::STRING),
                ) {
                    println!("\tsource: {}", source_title.get::<String>().unwrap());
                }
            }
        }

        let unlock_state = deployment.unlocked();
        if unlock_state != DeploymentUnlockedState::None {
            println!(
                "  unlocked: {}",
                match unlock_state {
                    DeploymentUnlockedState::Development => "development",
                    DeploymentUnlockedState::Hotfix => "hotfix",
                    DeploymentUnlockedState::None => "none",
                    DeploymentUnlockedState::Transient => "transient",
                    _ => "",
                }
            );
        }

        if deployment.is_pinned() {
            println!("  pinned: yes");
        }
        if let Some(origin) = deployment.origin() {
            match origin.string("origin", "refspec") {
                Err(_) => println!("  origin: unknown origin type"),
                Ok(refspec) => println!("  origin: {}", refspec),
            }
        }
    }

    Ok(())
}

fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}
