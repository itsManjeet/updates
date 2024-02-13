use clap::{ArgMatches, Command};
// use ostree::{COMMIT_META_KEY_SOURCE_TITLE, COMMIT_META_KEY_VERSION, DeploymentUnlockedState};
// use ostree::glib::{VariantDict, VariantTy};
use crate::{engine::Engine, Error};

pub fn cmd() -> Command {
    Command::new("status")
        .about("Show deployment status")
        .long_about("Check and apply system updates")
}

pub async fn run(_: &ArgMatches, engine: &Engine) -> Result<(), Error> {
    let booted_deployment = engine.sysroot.booted_deployment();
    for deployment in engine.states()?.iter() {
        let status = match &booted_deployment {
            Some(booted_deployment) => {
                if booted_deployment.csum() == deployment.revision {
                    "*"
                } else {
                    "-"
                }
            }
            None => "-",
        };
        println!(
            "{status} {}:{}",
            deployment.core.refspec,
            truncate(&deployment.core.revision, 6)
        );
        println!("    merged    : {}", deployment.merged);
        println!("    revision  : {}", truncate(&deployment.revision, 6));
        if deployment.extensions.len() > 0 {
            println!("    extensions: {}", deployment.extensions.len());
            for ext in &deployment.extensions {
                println!("        - {}:{}", ext.refspec, truncate(&ext.revision, 6));
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
