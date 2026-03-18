use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,           // 项目名称，如 "dev", "prod", "dev/mysql"
    #[serde(default)]
    pub prefix: String,        // Bitwarden 文件夹前缀，如 "dev", "prod"（可选）
    pub services: Vec<String>, // 该项目的服务列表
}

impl Project {
    pub fn new(name: impl Into<String>, prefix: impl Into<String>, services: Vec<String>) -> Self {
        Self {
            name: name.into(),
            prefix: prefix.into(),
            services,
        }
    }
}
