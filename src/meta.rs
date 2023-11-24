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
