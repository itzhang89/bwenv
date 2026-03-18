use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,           // Project name, e.g., "dev", "prod", "dev/mysql"
    #[serde(default)]
    pub prefix: String,        // Bitwarden folder prefix, e.g., "dev", "prod" (optional)
    #[serde(default)]
    pub services: Option<Vec<String>>, // Service list for this project (empty means query all)
}

impl Project {
    pub fn new(name: impl Into<String>, prefix: impl Into<String>, services: Option<Vec<String>>) -> Self {
        Self {
            name: name.into(),
            prefix: prefix.into(),
            services,
        }
    }
}
