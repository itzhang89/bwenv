pub mod models;
pub mod rules;

use anyhow::{anyhow, Result};
use models::Project;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BitwardenConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub master_password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub bitwarden: Option<BitwardenConfig>,
    /// 默认输出格式: shell, env, json
    #[serde(rename = "default_format")]
    pub default_format: Option<String>,
    /// 项目配置列表
    pub projects: Vec<Project>,
    /// 当前选中的项目名称
    #[serde(rename = "current_project")]
    pub current_project: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bitwarden: None,
            default_format: Some("shell".to_string()),
            projects: Vec::new(),
            current_project: None,
        }
    }
}

impl Config {
    /// 加载配置
    pub fn load() -> Result<Self> {
        Self::load_from_file()
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

    fn config_path() -> PathBuf {
        // ~/.bwenv 文件
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".bwenv")
    }

    /// 从当前目录或父目录查找 .bwenv 文件
    pub fn find_bwenv_in_dir() -> Option<PathBuf> {
        let current_dir = std::env::current_dir().ok()?;
        let mut path = current_dir.as_path();

        // 向上查找最多 5 层
        for _ in 0..5 {
            let bwenv_path = path.join(".bwenv");
            // 只查找文件，不查找目录
            if bwenv_path.is_file() {
                return Some(bwenv_path);
            }
            if let Some(parent) = path.parent() {
                path = parent;
            } else {
                break;
            }
        }
        None
    }

    /// 从 .bwenv 文件加载项目配置
    pub fn load_project_from_dir() -> Result<Option<Project>> {
        if let Some(path) = Self::find_bwenv_in_dir() {
            let content = fs::read_to_string(&path)?;
            let project: Project = serde_yaml::from_str(&content)?;
            Ok(Some(project))
        } else {
            Ok(None)
        }
    }

    /// 从文件加载项目配置
    pub fn load_projects_from_file(path: &str) -> Result<Vec<Project>> {
        let content = fs::read_to_string(path)?;
        // 尝试解析为 Vec<Project>，如果失败则尝试解析为单个 Project
        if let Ok(projects) = serde_yaml::from_str::<Vec<Project>>(&content) {
            Ok(projects)
        } else if let Ok(project) = serde_yaml::from_str::<Project>(&content) {
            Ok(vec![project])
        } else {
            Err(anyhow!("无法解析项目配置文件"))
        }
    }

    /// 保存配置到文件
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_yaml::to_string(&self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// 获取当前项目配置
    pub fn get_current_project(&self) -> Option<&Project> {
        self.current_project
            .as_ref()
            .and_then(|name| self.projects.iter().find(|p| &p.name == name))
    }

    /// 根据项目名获取项目配置
    pub fn get_project_by_name(&self, name: &str) -> Option<&Project> {
        self.projects.iter().find(|p| p.name == name)
    }

    /// 切换当前项目
    pub fn set_current_project(&mut self, project_name: &str) -> Result<()> {
        if !self.projects.iter().any(|p| p.name == project_name) {
            return Err(anyhow!("项目 '{}' 不存在", project_name));
        }
        self.current_project = Some(project_name.to_string());
        self.save()
    }

    /// 获取 Master Password
    pub fn get_master_password(&self) -> Option<&str> {
        self.bitwarden.as_ref().and_then(|b| b.master_password.as_deref())
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

    /// 设置默认输出格式
    pub fn set_default_format(&mut self, format: String) -> Result<()> {
        self.default_format = Some(format);
        self.save()
    }

    /// 添加项目
    pub fn add_project(&mut self, project: Project) -> Result<()> {
        if self.projects.iter().any(|p| p.name == project.name) {
            return Err(anyhow!("项目 '{}' 已存在", project.name));
        }
        self.projects.push(project);
        self.save()
    }

    /// 删除项目
    pub fn remove_project(&mut self, name: &str) -> Result<()> {
        let initial_len = self.projects.len();
        self.projects.retain(|p| p.name != name);
        if self.projects.len() == initial_len {
            return Err(anyhow!("项目 '{}' 不存在", name));
        }
        if self.current_project.as_deref() == Some(name) {
            self.current_project = None;
        }
        self.save()
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
