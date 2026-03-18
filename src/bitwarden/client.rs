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

    /// Get session cache file path
    fn session_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".bwenv_session")
    }

    /// Load cached session
    fn load_session() -> Option<String> {
        let path = Self::session_path();
        if path.exists() {
            fs::read_to_string(&path).ok()
        } else {
            None
        }
    }

    /// Save session to cache
    fn save_session(session: &str) -> Result<()> {
        let path = Self::session_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, session)?;
        Ok(())
    }

    /// Check if session is valid
    fn check_session(&mut self) -> Result<bool> {
        if let Some(ref session) = self.session_key {
            let output = Command::new("bw")
                .args(["list", "items", "--session", session])
                .output()?;

            if output.status.success() {
                return Ok(true);
            }
        }

        // Try to load cached session
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

    /// Unlock vault
    fn unlock(&mut self, password: &str) -> Result<()> {
        let output = Command::new("bw")
            .args(["unlock", password, "--raw"])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Unlock failed: {}", stderr));
        }

        let session = String::from_utf8_lossy(&output.stdout).trim().to_string();
        self.session_key = Some(session.clone());
        Self::save_session(&session)?;

        Ok(())
    }

    /// Ensure unlocked (try to unlock if not unlocked)
    /// If master_password is None and unlock is needed, returns error
    pub fn ensure_unlocked(&mut self, master_password: Option<&str>) -> Result<()> {
        if self.session_key.is_none() {
            // Try to check cached session
            if !self.check_session()? {
                // Need to unlock
                let password = master_password.ok_or_else(|| {
                    anyhow!("Vault is locked, need master password to unlock")
                })?;
                self.unlock(password)?;
            }
        } else {
            // Session exists, but may be expired, check status
            let status_output = Command::new("bw").arg("status").output()?;
            let status_str = String::from_utf8_lossy(&status_output.stdout);

            if status_str.contains("\"status\":\"locked\"") {
                // Session expired, vault is locked, need to re-unlock
                let password = master_password.ok_or_else(|| {
                    anyhow!("Vault is locked, need master password to unlock")
                })?;
                self.unlock(password)?;
            }
        }
        Ok(())
    }

    /// Get session (unlock if needed first)
    pub fn get_session(&mut self, master_password: Option<&str>) -> Result<String> {
        self.ensure_unlocked(master_password)?;
        self.session_key
            .clone()
            .ok_or_else(|| anyhow!("Cannot get session"))
    }

    /// List all items
    pub fn list_items(&mut self, master_password: Option<&str>) -> Result<Vec<BitwardenItem>> {
        let session = self.get_session(master_password)?;

        let output = Command::new("bw")
            .args(["list", "items", "--session", &session])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to get items: {}", stderr));
        }

        let items: Vec<BitwardenItem> = serde_json::from_slice(&output.stdout)?;
        Ok(items)
    }

    /// List all folders
    pub fn list_folders(&mut self, master_password: Option<&str>) -> Result<Vec<BitwardenFolder>> {
        let session = self.get_session(master_password)?;

        let output = Command::new("bw")
            .args(["list", "folders", "--session", &session])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to get folders: {}", stderr));
        }

        let folders: Vec<BitwardenFolder> = serde_json::from_slice(&output.stdout)?;
        Ok(folders)
    }

    /// Filter items by folder prefix and service name
    pub fn list_items_by_folder_and_service(
        &mut self,
        master_password: Option<&str>,
        folder_prefix: Option<&str>,
        service_name: Option<&str>,
    ) -> Result<Vec<BitwardenItem>> {
        let items = self.list_items(master_password)?;
        let folders = self.list_folders(master_password)?;

        // Build folder_id -> folder_name mapping
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
                // Get folder name
                let folder_name = item
                    .folder_id
                    .as_str()
                    .and_then(|id| folder_map.get(id))
                    .map(|s| s.as_str())
                    .unwrap_or("");

                // Filter by folder prefix
                let matches_prefix = if let Some(prefix) = folder_prefix {
                    folder_name.starts_with(prefix)
                } else {
                    true
                };

                // Filter by service name
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
