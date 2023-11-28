use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct MetaInfo {
    pub id: String,
    pub version: String,
    pub about: String,
    pub depends: Vec<String>,
    pub integration: String,
    pub cache: String,
}

impl Hash for MetaInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let serialized = serde_yaml::to_string(&self).unwrap();
        serialized.hash(state);
    }
}
