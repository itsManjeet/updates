use merge_struct::merge;
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Split {
    pub id: String,
    pub pattern: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Element {
    pub id: Option<String>,
    pub version: Option<String>,
    pub about: Option<String>,

    pub depends: Option<Vec<String>>,
    pub integration: Option<String>,

    pub build_depends: Option<Vec<String>>,

    pub config: Option<HashMap<String, String>>,
    pub sources: Option<Vec<String>>,
    pub environ: Option<Vec<String>>,

    pub split: Option<Vec<Split>>,

    import: Option<Vec<String>>,
}

impl Element {
    pub fn open(filepath: &PathBuf, search_path: Option<&PathBuf>) -> Result<Element, Error> {
        let reader = File::open(&filepath)?;
        let mut element: Element = serde_yaml::from_reader(reader)?;
        if let Some(imports) = element.import.clone() {
            for i in imports.iter() {
                let include_path = match search_path {
                    Some(p) => p.join(i),
                    None => PathBuf::from(i),
                };
                let child = Element::open(&include_path, search_path)?;
                let merged = match merge(&child, &element) {
                    Ok(merged) => merged,
                    Err(_) => return Err(Error::MergeFailed),
                };
                element = merged;
            }
        }

        Ok(element)
    }

    pub fn resolve(&mut self, environ: &Vec<String>, variables: &HashMap<String, String>) {
        let mut table = variables.clone();
        if let Some(config) = &self.config {
            table.extend(config.clone());
        }

        if let Some(env) = &mut self.environ {
            env.into_iter().chain(environ.into_iter());
        }

        let re = Regex::new(r"\%\{([a-zA-Z_][0-9a-zA-Z_]*)\}").unwrap();
        if let Some(config) = &self.config {
            for (_, value) in config.iter() {
                re.replace_all(&value, |caps: &Captures| {
                    if variables.contains_key(&caps[1]) {
                        return variables[&caps[1]].clone();
                    }
                    return String::from("");
                });
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("system io error")]
    IOError(#[from] std::io::Error),

    #[error("yaml parsing failed")]
    YamlParsingFailed(#[from] serde_yaml::Error),

    #[error("merge failed")]
    MergeFailed,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::Element;

    #[test]
    fn test_open() {
        let testfiles = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("elements");

        let element = Element::open(&testfiles.join("components/acl.yml"), Some(&testfiles))
            .expect("failed to open element");

        assert_eq!("acl", element.id.unwrap());
        assert_eq!("0.0.2", element.version.unwrap());
    }
}
