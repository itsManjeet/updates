use merge_struct::merge;
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
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
    pub environ: Option<HashMap<String, String>>,

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

    pub fn resolve(
        &self,
        environ: &HashMap<String, String>,
        variables: &HashMap<String, String>,
    ) -> Result<Element, Error> {
        let mut config = variables.clone();
        if let Some(c) = &self.config {
            config.extend(c.clone());
        }
        if let Some(id) = &self.id {
            config.insert(String::from("id"), id.clone());
        }
        if let Some(version) = &self.version {
            config.insert(String::from("version"), version.clone());
        }

        let serialized = serde_yaml::to_string(&self).unwrap();

        let re = Regex::new(r"\%\{([a-zA-Z_][0-9a-zA-Z_]*)\}").unwrap();
        let serialized = re.replace_all(&serialized, |caps: &Captures| {
            if config.contains_key(&caps[1]) {
                return config[&caps[1]].clone();
            }
            return String::from("");
        });

        let mut element: Element = serde_yaml::from_str(&serialized).unwrap();
        if let Some(e) = &mut element.environ {
            e.extend(environ.clone());
        }

        Ok(element)
    }
}

impl Hash for Element {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let serialized = serde_yaml::to_string(&self).unwrap();
        serialized.hash(state);
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
    use std::{collections::HashMap, path::PathBuf};

    use super::Element;

    #[test]
    fn test_open() {
        let testfiles = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");

        let element = Element::open(&testfiles.join("elements/sample.yml"), Some(&testfiles))
            .expect("failed to open element");

        assert_eq!("sample", element.id.unwrap());
        assert_eq!("0.0.1", element.version.unwrap());
    }

    #[test]
    fn test_resolve() {
        let testfiles = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");

        let element = Element::open(&testfiles.join("elements/sample.yml"), Some(&testfiles))
            .expect("failed to open element");

        let environ: HashMap<String, String> = HashMap::new();
        let variables: HashMap<String, String> = HashMap::new();

        let element = element.resolve(&environ, &variables).unwrap();

        assert_eq!("here is sample 0.0.1", element.config.unwrap()["script"]);
    }
}
