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

    /// bwenv data directory (session and future local state), e.g. `~/.bwenv.d`
    fn data_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".bwenv.d")
    }

    /// Session cache file (bwenv-only; not used by the `bw` CLI)
    fn session_path() -> PathBuf {
        Self::data_dir().join("session")
    }

    fn read_session_file(path: &std::path::Path) -> Option<String> {
        fs::read_to_string(path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    /// Load cached session (trimmed; file may end with newline)
    fn load_session() -> Option<String> {
        let path = Self::session_path();
        if path.exists() {
            return Self::read_session_file(&path);
        }
        None
    }

    /// True if `bw list items` accepts this session and returns non-empty JSON-like output.
    /// Exit code alone is not enough: some failures still exit 0 with an empty stdout.
    fn session_can_list_items(session: &str) -> Result<bool> {
        let output = Command::new("bw")
            .args(["list", "items", "--session", session])
            .output()?;

        if !output.status.success() {
            return Ok(false);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();
        if trimmed.is_empty() {
            return Ok(false);
        }
        Ok(trimmed.starts_with('[') || trimmed.starts_with('{'))
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

    /// Clear session file (new and legacy paths)
    fn clear_session() {
        let path = Self::session_path();
        if path.exists() {
            let _ = fs::remove_file(&path);
        }
    }

    /// Check if error indicates invalid/expired session
    fn is_session_invalid_error(error: &str) -> bool {
        let lower = error.to_lowercase();
        lower.contains("empty response")
            || lower.contains("not json")
            || lower.contains("session")
            || lower.contains("unauthorized")
            || lower.contains("not authorized")
            || lower.contains("bw_errorresponse")
            || lower.contains("too many login requests")
    }

    /// Check if session is valid (must actually list items with parseable-looking output).
    fn check_session(&mut self) -> Result<bool> {
        if let Some(ref session) = self.session_key {
            if Self::session_can_list_items(session)? {
                return Ok(true);
            }
            // In-memory session is stale
            self.session_key = None;
        }

        // Try to load cached session from disk
        if let Some(session) = Self::load_session() {
            if Self::session_can_list_items(&session)? {
                self.session_key = Some(session);
                return Ok(true);
            }
            // Drop bad cache so later unlock writes a fresh key
            Self::clear_session();
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

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            return Err(anyhow!("Empty response from Bitwarden. Is the vault locked or empty?"));
        }

        // Check if response looks like valid JSON before parsing
        let trimmed = stdout.trim();
        if !trimmed.starts_with('[') && !trimmed.starts_with('{') {
            return Err(anyhow!(
                "Invalid response from Bitwarden (not JSON): {}. Is the vault locked?",
                trimmed.chars().take(200).collect::<String>()
            ));
        }

        let items: Vec<BitwardenItem> = serde_json::from_slice(&output.stdout).map_err(|e| {
            anyhow!("Failed to parse items JSON: {}. Response: {}", e, trimmed.chars().take(500).collect::<String>())
        })?;
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

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            return Err(anyhow!("Empty response from Bitwarden. Is the vault locked or empty?"));
        }

        // Check if response looks like valid JSON before parsing
        let trimmed = stdout.trim();
        if !trimmed.starts_with('[') && !trimmed.starts_with('{') {
            return Err(anyhow!(
                "Invalid response from Bitwarden (not JSON): {}. Is the vault locked?",
                trimmed.chars().take(200).collect::<String>()
            ));
        }

        let folders: Vec<BitwardenFolder> = serde_json::from_slice(&output.stdout).map_err(|e| {
            anyhow!("Failed to parse folders JSON: {}. Response: {}", e, trimmed.chars().take(500).collect::<String>())
        })?;
        Ok(folders)
    }

    /// Filter items by folder prefix and service name
    pub fn list_items_by_folder_and_service(
        &mut self,
        master_password: Option<&str>,
        folder_prefix: Option<&str>,
        service_name: Option<&str>,
    ) -> Result<Vec<BitwardenItem>> {
        self.list_items_by_folder_and_service_impl(master_password, folder_prefix, service_name, true)
    }

    /// Internal implementation with retry support
    fn list_items_by_folder_and_service_impl(
        &mut self,
        master_password: Option<&str>,
        folder_prefix: Option<&str>,
        service_name: Option<&str>,
        is_retry: bool,
    ) -> Result<Vec<BitwardenItem>> {
        let items_result = self.list_items(master_password);
        let folders_result = self.list_folders(master_password);

        // Check if either call failed with session-related error
        let items_err_str = items_result.as_ref().err().map(|e| e.to_string()).unwrap_or_default();
        let folders_err_str = folders_result.as_ref().err().map(|e| e.to_string()).unwrap_or_default();

        // If failed due to session issue and we haven't retried yet, try re-unlocking
        if (Self::is_session_invalid_error(&items_err_str) || Self::is_session_invalid_error(&folders_err_str))
            && !is_retry
            && master_password.is_some()
        {
            // Clear invalid session and re-unlock
            Self::clear_session();
            self.session_key = None;

            // Re-unlock with master password
            if let Err(e) = self.unlock(master_password.unwrap()) {
                return Err(anyhow!("Session expired, re-unlock failed: {}", e));
            }

            // Retry the operation once
            return self.list_items_by_folder_and_service_impl(
                master_password,
                folder_prefix,
                service_name,
                true,
            );
        }

        let items = items_result?;
        let folders = folders_result?;

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
