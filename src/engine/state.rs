use std::env;

use ostree::{Deployment, ObjectType, Repo};
use ostree::glib::{GString, VariantDict, VariantTy};

use crate::Error;

pub struct RefState {
    pub refspec: String,
    pub revision: String,
}

pub struct State {
    pub core: RefState,
    pub merged: bool,
    pub extensions: Option<Vec<RefState>>,
}


impl State {
    pub fn options(&self) -> (VariantDict, String) {
        let options = VariantDict::new(None);
        let extensions_string: String = "".to_string();
        options.insert("rlxos.revision.core", &self.core.revision);
        if self.merged {
            if let Some(extensions) = &self.extensions {
                for extension in extensions {
                    let extension_id = extension.refspec.to_string().split("/").map(|s| s.to_string()).collect::<Vec<String>>()[2].clone();
                    options.insert(&format!("rlxos.revision.{}", extension_id), &extension.revision);
                }
            }
        }
        (options, extensions_string)
    }
    pub fn for_deployment(repo: &Repo, deployment: &Deployment) -> Result<State, Error> {
        let origin = deployment.origin().unwrap();
        let refspec = origin.string("origin", "refspec").unwrap_or_else(|_| { "rlxos:x86_64/os/stable".into() }).to_string();
        let revision = deployment.csum().to_string();
        let merged = origin.boolean("rlxos", "merged").unwrap_or_else(|_| false);

        if !merged {
            return Ok(State {
                core: RefState { refspec, revision },
                merged,
                extensions: None,
            });
        }

        let channel = origin.string("rlxos", "channel").unwrap_or_else(|_| "stable".into()).to_string();

        let refspec = format!("{}:{}/os/{}", deployment.osname(), env::consts::ARCH, channel);

        let commit = repo.load_variant(ObjectType::Commit, &revision)?;
        let commit_metadata = VariantDict::new(Some(&commit.child_value(0)));

        let revision = get_revision(&commit_metadata, "core");


        let extensions_refspec: Vec<String> = origin.string("rlxos", "extensions").unwrap_or_else(|_| GString::from("")).to_string().split(";").map(|s| s.to_string()).collect();
        let mut extensions: Vec<RefState> = Vec::new();
        for ext in extensions_refspec.clone() {
            // Skip previous extensions
            if ext.contains("/extension/") {
                continue;
            }
            let ext_refspec = format!("{}:{}/extension/{}", deployment.osname(), env::consts::ARCH, channel);
            let ext_revision = get_revision(&commit_metadata, &ext);
            extensions.push(RefState {
                refspec: ext_refspec,
                revision: ext_revision,
            });
        }

        Ok(State {
            core: RefState { refspec, revision },
            merged,
            extensions: Some(extensions),
        })
    }
}

fn get_revision(metadata: &VariantDict, id: &str) -> String {
    match metadata.lookup_value(format!("rlxos.revision.{}", id).as_str(), Some(&VariantTy::STRING)) {
        Some(variant) => variant.get::<String>().unwrap(),
        None => "".into(),
    }
}