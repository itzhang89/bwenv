pub mod models;
pub mod rules;

use anyhow::{anyhow, Result};
use models::Project;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BitwardenConfig {
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub master_password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub bitwarden: Option<BitwardenConfig>,
    #[serde(rename = "default_prefix")]
    pub default_prefix: Option<String>,
    #[serde(rename = "default_format")]
    pub default_format: Option<String>,
    #[serde(rename = "default_services")]
    pub default_services: Vec<String>,
    pub projects: Vec<Project>,
    #[serde(rename = "current_project")]
    pub current_project: Option<String>,
    #[serde(rename = "config_file")]
    pub config_file: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bitwarden: None,
            default_prefix: None,
            default_format: Some("shell".to_string()),
            default_services: Vec::new(),
            projects: Vec::new(),
            current_project: None,
            config_file: None,
        }
    }
}

impl Config {
    /// 加载配置，优先级：命令行 > 环境变量 > ~/.bwenv/config.yaml
    pub fn load() -> Result<Self> {
        // 1. 加载 ~/.bwenv/config.yaml
        let file_config = Self::load_from_file();

        // 2. 加载环境变量覆盖
        let env_config = Self::load_from_env();

        // 3. 合并配置（环境变量优先）
        let mut config = file_config.unwrap_or_default();

        if let Some(env) = env_config {
            if let Some(bitwarden) = env.bitwarden {
                if let Some(api_key) = bitwarden.api_key {
                    if config.bitwarden.is_none() {
                        config.bitwarden = Some(BitwardenConfig::default());
                    }
                    config.bitwarden.as_mut().unwrap().api_key = Some(api_key);
                }
                if let Some(api_secret) = bitwarden.api_secret {
                    if config.bitwarden.is_none() {
                        config.bitwarden = Some(BitwardenConfig::default());
                    }
                    config.bitwarden.as_mut().unwrap().api_secret = Some(api_secret);
                }
            }
            if let Some(prefix) = env.default_prefix {
                config.default_prefix = Some(prefix);
            }
            if let Some(format) = env.default_format {
                config.default_format = Some(format);
            }
        }

        Ok(config)
    }

    fn load_from_file() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let config: Config = serde_yaml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    fn load_from_env() -> Option<Self> {
        let api_key = std::env::var("BWENV_API_KEY").ok();
        let api_secret = std::env::var("BWENV_API_SECRET").ok();
        let default_prefix = std::env::var("BWENV_PREFIX").ok();
        let default_format = std::env::var("BWENV_FORMAT").ok();
        let config_file = std::env::var("BWENV_CONFIG").ok();

        if api_key.is_none()
            && api_secret.is_none()
            && default_prefix.is_none()
            && default_format.is_none()
            && config_file.is_none()
        {
            return None;
        }

        Some(Config {
            bitwarden: if api_key.is_some() || api_secret.is_some() {
                Some(BitwardenConfig {
                    api_key,
                    api_secret,
                    master_password: None,
                })
            } else {
                None
            },
            default_prefix,
            default_format,
            default_services: Vec::new(),
            projects: Vec::new(),
            current_project: None,
            config_file,
        })
    }

    fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".bwenv")
            .join("config.yaml")
    }

    /// 保存配置到文件
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();

        // 确保目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_yaml::to_string(&self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// 切换当前项目
    pub fn set_current_project(&mut self, project_name: &str) -> Result<()> {
        // 验证项目是否存在
        if !self.projects.iter().any(|p| p.name == project_name) {
            return Err(anyhow!("项目 '{}' 不存在", project_name));
        }
        self.current_project = Some(project_name.to_string());
        self.save()?;
        Ok(())
    }

    /// 获取当前项目的配置
    pub fn get_current_project(&self) -> Option<&Project> {
        self.current_project
            .as_ref()
            .and_then(|name| self.projects.iter().find(|p| &p.name == name))
    }

    /// 根据项目名获取项目配置
    pub fn get_project_by_name(&self, name: &str) -> Option<&Project> {
        self.projects.iter().find(|p| p.name == name)
    }

    /// 获取 API Key
    pub fn get_api_key(&self) -> Option<&str> {
        self.bitwarden.as_ref().and_then(|b| b.api_key.as_deref())
    }

    /// 获取 API Secret
    pub fn get_api_secret(&self) -> Option<&str> {
        self.bitwarden.as_ref().and_then(|b| b.api_secret.as_deref())
    }

    /// 获取 Master Password
    pub fn get_master_password(&self) -> Option<&str> {
        self.bitwarden.as_ref().and_then(|b| b.master_password.as_deref())
    }

    /// 设置 API Key
    pub fn set_api_key(&mut self, api_key: String) -> Result<()> {
        if self.bitwarden.is_none() {
            self.bitwarden = Some(BitwardenConfig::default());
        }
        self.bitwarden.as_mut().unwrap().api_key = Some(api_key);
        self.save()
    }

    /// 设置 API Secret
    pub fn set_api_secret(&mut self, api_secret: String) -> Result<()> {
        if self.bitwarden.is_none() {
            self.bitwarden = Some(BitwardenConfig::default());
        }
        self.bitwarden.as_mut().unwrap().api_secret = Some(api_secret);
        self.save()
    }

    /// 设置 Master Password
    pub fn set_master_password(&mut self, master_password: String) -> Result<()> {
        if self.bitwarden.is_none() {
            self.bitwarden = Some(BitwardenConfig::default());
        }
        self.bitwarden.as_mut().unwrap().master_password = Some(master_password);
        self.save()
    }

    /// 获取默认输出格式
    #[allow(dead_code)]
    pub fn get_default_format(&self) -> &str {
        self.default_format
            .as_deref()
            .unwrap_or("shell")
    }
}

/// 从配置文件加载服务列表
pub fn load_services_from_file(path: &str) -> Result<Vec<String>> {
    let content = fs::read_to_string(path)?;
    let services: Vec<String> = content
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    Ok(services)
}
