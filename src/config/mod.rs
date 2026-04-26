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
    /// Default output format: shell, env, json
    #[serde(rename = "default_format")]
    pub default_format: Option<String>,
    /// Project configuration list
    pub projects: Vec<Project>,
    /// Current selected project name
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
    /// Load configuration
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
        // ~/.bwenv.d/bwenv
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".bwenv.d/bwenv")
    }

    /// Find .bwenv file in current or parent directory
    pub fn find_bwenv_in_dir() -> Option<PathBuf> {
        let current_dir = std::env::current_dir().ok()?;
        let mut path = current_dir.as_path();

        // Search up to 5 levels
        for _ in 0..5 {
            let bwenv_path = path.join(".bwenv");
            // Only look for files, not directories
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

    /// Load project configuration from .bwenv file
    pub fn load_project_from_dir() -> Result<Option<Project>> {
        if let Some(path) = Self::find_bwenv_in_dir() {
            let content = fs::read_to_string(&path)?;
            let project: Project = serde_yaml::from_str(&content)?;
            Ok(Some(project))
        } else {
            Ok(None)
        }
    }

    /// Load project configurations from file
    pub fn load_projects_from_file(path: &str) -> Result<Vec<Project>> {
        let content = fs::read_to_string(path)?;
        // Try to parse as Vec<Project>, if failed try single Project
        if let Ok(projects) = serde_yaml::from_str::<Vec<Project>>(&content) {
            Ok(projects)
        } else if let Ok(project) = serde_yaml::from_str::<Project>(&content) {
            Ok(vec![project])
        } else {
            Err(anyhow!("Cannot parse project config file"))
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_yaml::to_string(&self)?;
        fs::write(&path, content)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(|e| {
                anyhow!("failed to set mode 0600 on {}: {}", path.display(), e)
            })?;
        }

        Ok(())
    }

    /// Get current project configuration
    pub fn get_current_project(&self) -> Option<&Project> {
        self.current_project
            .as_ref()
            .and_then(|name| self.projects.iter().find(|p| &p.name == name))
    }

    /// Get project configuration by name
    #[allow(dead_code)]
    pub fn get_project_by_name(&self, name: &str) -> Option<&Project> {
        self.projects.iter().find(|p| p.name == name)
    }

    /// Switch current project
    pub fn set_current_project(&mut self, project_name: &str) -> Result<()> {
        if !self.projects.iter().any(|p| p.name == project_name) {
            return Err(anyhow!("Project '{}' does not exist", project_name));
        }
        self.current_project = Some(project_name.to_string());
        self.save()
    }

    /// Get Master Password
    pub fn get_master_password(&self) -> Option<&str> {
        self.bitwarden.as_ref().and_then(|b| b.master_password.as_deref())
    }

    /// Set Master Password
    pub fn set_master_password(&mut self, master_password: String) -> Result<()> {
        if self.bitwarden.is_none() {
            self.bitwarden = Some(BitwardenConfig::default());
        }
        self.bitwarden.as_mut().unwrap().master_password = Some(master_password);
        self.save()
    }

    /// Get default output format
    #[allow(dead_code)]
    pub fn get_default_format(&self) -> &str {
        self.default_format
            .as_deref()
            .unwrap_or("shell")
    }

    /// Set default output format
    pub fn set_default_format(&mut self, format: String) -> Result<()> {
        self.default_format = Some(format);
        self.save()
    }

    /// Add project
    pub fn add_project(&mut self, project: Project) -> Result<()> {
        if self.projects.iter().any(|p| p.name == project.name) {
            return Err(anyhow!("Project '{}' already exists", project.name));
        }
        self.projects.push(project);
        self.save()
    }

    /// Remove project
    pub fn remove_project(&mut self, name: &str) -> Result<()> {
        let initial_len = self.projects.len();
        self.projects.retain(|p| p.name != name);
        if self.projects.len() == initial_len {
            return Err(anyhow!("Project '{}' does not exist", name));
        }
        if self.current_project.as_deref() == Some(name) {
            self.current_project = None;
        }
        self.save()
    }
}

/// Load service list from config file
pub fn load_services_from_file(path: &str) -> Result<Vec<String>> {
    let content = fs::read_to_string(path)?;
    let services: Vec<String> = content
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    Ok(services)
}
