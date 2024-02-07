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
    for deployment in engine.states()?.iter() {
        println!("{}", deployment.core.refspec);
    }

    Ok(())
}

// fn truncate(s: &str, max_chars: usize) -> &str {
//     match s.char_indices().nth(max_chars) {
//         None => s,
//         Some((idx, _)) => &s[..idx],
//     }
// }
