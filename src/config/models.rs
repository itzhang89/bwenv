use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,           // 项目名称，如 "dev/project1"
    pub prefix: String,         // 前缀，如 "dev"
    pub services: Vec<String>,  // 该项目的服务列表
}

impl Project {
    #[allow(dead_code)]
    pub fn new(name: impl Into<String>, prefix: impl Into<String>, services: Vec<String>) -> Self {
        Self {
            name: name.into(),
            prefix: prefix.into(),
            services,
        }
    }
}
