use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::bitwarden::models::{BitwardenFolder, BitwardenItem};

pub struct BitwardenClient {
    session_key: Option<String>,
}

impl BitwardenClient {
    pub fn new() -> Self {
        Self { session_key: None }
    }

    /// 获取 session 缓存文件路径
    fn session_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".bwenv")
            .join(".session")
    }

    /// 加载缓存的 session
    fn load_session() -> Option<String> {
        let path = Self::session_path();
        if path.exists() {
            fs::read_to_string(&path).ok()
        } else {
            None
        }
    }

    /// 保存 session 到缓存
    fn save_session(session: &str) -> Result<()> {
        let path = Self::session_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, session)?;
        Ok(())
    }

    /// 检查 session 是否有效
    fn check_session(&mut self) -> Result<bool> {
        if let Some(ref session) = self.session_key {
            let output = Command::new("bw")
                .args(["list", "items", "--session", session])
                .output()?;

            if output.status.success() {
                return Ok(true);
            }
        }

        // 尝试加载缓存的 session
        if let Some(session) = Self::load_session() {
            let output = Command::new("bw")
                .args(["list", "items", "--session", &session])
                .output()?;

            if output.status.success() {
                self.session_key = Some(session);
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// 登录 Bitwarden
    pub fn login(&mut self, api_key: &str, api_secret: &str, master_password: Option<&str>) -> Result<()> {
        // 检查当前状态
        let status_output = Command::new("bw").arg("status").output()?;
        let status_str = String::from_utf8_lossy(&status_output.stdout);

        // 如果已经登录但锁定，尝试解锁
        if status_str.contains("\"status\":\"locked\"") {
            if let Some(password) = master_password {
                return self.unlock(password);
            } else {
                return Err(anyhow!("保险库已锁定，请提供主密码或运行 'bw unlock'"));
            }
        }

        // 如果未登录，先登录
        if status_str.contains("\"status\":\"unauthenticated\"") {
            // 使用 API key 登录 - 设置环境变量后调用 login --apikey
            let output = Command::new("bw")
                .args(["login", "--apikey"])
                .env("BW_CLIENTID", api_key)
                .env("BW_CLIENTSECRET", api_secret)
                .output()?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow!("登录失败: {}", stderr));
            }
        }

        // 解锁保险库
        if let Some(password) = master_password {
            self.unlock(password)?;
        } else {
            return Err(anyhow!("请提供主密码解锁保险库"));
        }

        Ok(())
    }

    /// 解锁保险库
    fn unlock(&mut self, password: &str) -> Result<()> {
        let output = Command::new("bw")
            .args(["unlock", password, "--raw"])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("解锁失败: {}", stderr));
        }

        let session = String::from_utf8_lossy(&output.stdout).trim().to_string();
        self.session_key = Some(session.clone());
        Self::save_session(&session)?;

        Ok(())
    }

    /// 确保已登录（如果未登录则尝试登录）
    pub fn ensure_logged_in(&mut self, api_key: &str, api_secret: &str, master_password: Option<&str>) -> Result<()> {
        if self.session_key.is_none() {
            // 尝试检查缓存 session
            if !self.check_session()? {
                // 需要登录
                self.login(api_key, api_secret, master_password)?;
            }
        } else {
            // session 存在，但可能已过期，检查状态
            let status_output = Command::new("bw").arg("status").output()?;
            let status_str = String::from_utf8_lossy(&status_output.stdout);

            if status_str.contains("\"status\":\"locked\"") {
                // session 过期，保险库已锁定，需要重新解锁
                if let Some(password) = master_password {
                    self.unlock(password)?;
                } else {
                    return Err(anyhow!("保险库已锁定，请提供主密码"));
                }
            }
        }
        Ok(())
    }

    /// 获取 session（如果需要先登录）
    pub fn get_session(&mut self, api_key: &str, api_secret: &str, master_password: Option<&str>) -> Result<String> {
        self.ensure_logged_in(api_key, api_secret, master_password)?;
        self.session_key
            .clone()
            .ok_or_else(|| anyhow!("无法获取 session"))
    }

    /// 列出所有 items
    pub fn list_items(&mut self, api_key: &str, api_secret: &str, master_password: Option<&str>) -> Result<Vec<BitwardenItem>> {
        let session = self.get_session(api_key, api_secret, master_password)?;

        let output = Command::new("bw")
            .args(["list", "items", "--session", &session])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("获取 items 失败: {}", stderr));
        }

        let items: Vec<BitwardenItem> = serde_json::from_slice(&output.stdout)?;
        Ok(items)
    }

    /// 列出所有 folders
    pub fn list_folders(&mut self, api_key: &str, api_secret: &str, master_password: Option<&str>) -> Result<Vec<BitwardenFolder>> {
        let session = self.get_session(api_key, api_secret, master_password)?;

        let output = Command::new("bw")
            .args(["list", "folders", "--session", &session])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("获取 folders 失败: {}", stderr));
        }

        let folders: Vec<BitwardenFolder> = serde_json::from_slice(&output.stdout)?;
        Ok(folders)
    }

    /// 根据前缀筛选 items
    #[allow(dead_code)]
    pub fn list_items_by_prefix(
        &mut self,
        api_key: &str,
        api_secret: &str,
        master_password: Option<&str>,
        prefix: Option<&str>,
        service_name: Option<&str>,
    ) -> Result<Vec<BitwardenItem>> {
        let items = self.list_items(api_key, api_secret, master_password)?;

        let filtered: Vec<BitwardenItem> = items
            .into_iter()
            .filter(|item| {
                // 按前缀筛选（folder name）
                let _prefix_matches = if let Some(_prefix) = prefix {
                    // 需要匹配 folder 的前缀
                    true // 暂时不做folder匹配，后续优化
                } else {
                    true
                };

                // 按服务名筛选
                let matches_service = if let Some(service) = service_name {
                    item.name.as_str().map(|n| n.to_lowercase()).unwrap_or_default().contains(&service.to_lowercase())
                } else {
                    true
                };

                _prefix_matches && matches_service
            })
            .collect();

        Ok(filtered)
    }

    /// 根据 folder 前缀和服务名筛选 items
    pub fn list_items_by_folder_and_service(
        &mut self,
        api_key: &str,
        api_secret: &str,
        master_password: Option<&str>,
        folder_prefix: Option<&str>,
        service_name: Option<&str>,
    ) -> Result<Vec<BitwardenItem>> {
        let items = self.list_items(api_key, api_secret, master_password)?;
        let folders = self.list_folders(api_key, api_secret, master_password)?;

        // 构建 folder_id -> folder_name 映射
        let folder_map: std::collections::HashMap<String, String> = folders
            .iter()
            .filter_map(|f| {
                let id = f.id.as_str()?;
                let name = f.name.as_str()?;
                Some((id.to_string(), name.to_string()))
            })
            .collect();

        let filtered: Vec<BitwardenItem> = items
            .into_iter()
            .filter(|item| {
                // 获取 folder name
                let folder_name = item
                    .folder_id
                    .as_str()
                    .and_then(|id| folder_map.get(id))
                    .map(|s| s.as_str())
                    .unwrap_or("");

                // 按 folder 前缀筛选
                let matches_prefix = if let Some(prefix) = folder_prefix {
                    folder_name.starts_with(prefix)
                } else {
                    true
                };

                // 按服务名筛选
                let matches_service = if let Some(service) = service_name {
                    item.name.as_str().map(|n| n.to_lowercase()).unwrap_or_default().contains(&service.to_lowercase())
                } else {
                    true
                };

                matches_prefix && matches_service
            })
            .collect();

        Ok(filtered)
    }
}
