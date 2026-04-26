use anyhow::{anyhow, Result};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::bitwarden::models::{BitwardenFolder, BitwardenItem};

pub struct BitwardenClient {
    session_key: Option<String>,
}

impl BitwardenClient {
    pub fn new() -> Self {
        let mut this = Self { session_key: None };
        // Priority: if caller already has a session in env, persist it for reuse.
        // Fallback: reuse cached session to avoid prompting/unlock on every run.
        if this.load_session_from_env().is_err() {
            // ignore
        }
        if this.session_key.is_none() {
            let _ = this.load_cached_session();
        }
        this
    }

    /// bwenv data directory (session and future local state), e.g. `~/.bwenv.d`
    fn data_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".bwenv.d")
    }

    fn session_path() -> PathBuf {
        Self::data_dir().join("session")
    }

    fn load_session_from_env(&mut self) -> Result<()> {
        let v = match env::var("BW_SESSION") {
            Ok(v) => v,
            Err(env::VarError::NotPresent) => return Ok(()),
            Err(e) => return Err(anyhow!("failed to read BW_SESSION env var: {}", e)),
        };

        let v = v.trim();
        if v.is_empty() {
            return Ok(());
        }

        self.session_key = Some(v.to_string());
        self.save_cached_session()?;
        Ok(())
    }

    fn load_cached_session(&mut self) -> Result<()> {
        let path = Self::session_path();
        if !path.exists() {
            return Ok(());
        }
        let s = fs::read_to_string(&path)
            .map_err(|e| anyhow!("failed to read cached session {}: {}", path.display(), e))?;
        let s = s.trim();
        if s.is_empty() {
            return Ok(());
        }
        self.session_key = Some(s.to_string());
        Ok(())
    }

    fn save_cached_session(&self) -> Result<()> {
        let session = match self.session_key.as_deref() {
            Some(s) if !s.trim().is_empty() => s.trim(),
            _ => return Ok(()),
        };

        let dir = Self::data_dir();
        fs::create_dir_all(&dir)
            .map_err(|e| anyhow!("failed to create data dir {}: {}", dir.display(), e))?;

        let path = Self::session_path();
        fs::write(&path, session)
            .map_err(|e| anyhow!("failed to write cached session {}: {}", path.display(), e))?;

        // Best-effort permissions tightening on unix.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
        }

        Ok(())
    }

    fn clear_cached_session(&mut self) {
        self.session_key = None;
        let _ = fs::remove_file(Self::session_path());
    }

    fn fetch_vault_status_json(&self, use_session: bool) -> Result<serde_json::Value> {
        let mut cmd = Command::new("bw");
        cmd.arg("status");
        if use_session {
            if let Some(ref s) = self.session_key {
                cmd.args(["--session", s]);
            }
        }

        let output = cmd.output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("bw status failed: {}", stderr));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("bw status: empty output"));
        }
        serde_json::from_str(trimmed).map_err(|e| anyhow!("bw status: invalid JSON: {}", e))
    }

    fn vault_status_str(value: &serde_json::Value) -> Result<&str> {
        value
            .get("status")
            .and_then(|s| s.as_str())
            .ok_or_else(|| anyhow!("bw status: missing \"status\" field"))
    }

    /// Unlock vault; sets `session_key` and caches it for locked-mode subprocesses.
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
        self.save_cached_session()?;

        Ok(())
    }

    /// Align with `bw status` for the current machine / CLI data dir:
    /// - `unauthenticated` → `bw login`, then `bw unlock` (needs master password).
    /// - `locked` → `bw unlock` (needs master password).
    /// - `unlocked` → no unlock;
    pub fn ensure_unlocked(&mut self, master_password: Option<&str>) -> Result<()> {
        // Prefer validating cached session first (fast path).
        // If it fails (expired/invalid), fall back to non-session status and re-unlock.
        let (status_json, used_session) = match self.session_key.as_deref() {
            Some(_) => match self.fetch_vault_status_json(true) {
                Ok(v) => (v, true),
                Err(_) => {
                    // cached session likely expired; clear and continue with normal flow
                    self.clear_cached_session();
                    (self.fetch_vault_status_json(false)?, false)
                }
            },
            None => (self.fetch_vault_status_json(false)?, false),
        };

        let status = Self::vault_status_str(&status_json)?;

        match status {
            "unauthenticated" => {
                let st = Command::new("bw")
                    .arg("login")
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .status()
                    .map_err(|e| anyhow!("failed to run bw login: {}", e))?;
                if !st.success() {
                    return Err(anyhow!("`bw login` exited with status {:?}", st.code()));
                }
                let password = master_password.ok_or_else(|| {
                    anyhow!("Master password required to unlock vault after login")
                })?;
                self.clear_cached_session();
                self.unlock(password)?;
            }
            "locked" => {
                let password = master_password.ok_or_else(|| {
                    anyhow!("Vault is locked; master password required to unlock")
                })?;
                self.clear_cached_session();
                self.unlock(password)?;
            }
            "unlocked" => {}
            other => return Err(anyhow!("Unknown Bitwarden vault status: {}", other)),
        }

        // If we reached unlocked via non-session flow, we might still benefit from a cached session
        // for subsequent bw commands. If we don't have one, try to unlock (requires master password).
        if !used_session && self.session_key.is_none() {
            if let Some(password) = master_password {
                // This will no-op if already unlocked? `bw unlock --raw` still returns a session.
                let _ = self.unlock(password);
            }
        }

        Ok(())
    }

    fn bw_cmd_list_items(&mut self, master_password: Option<&str>) -> Result<std::process::Output> {
        self.ensure_unlocked(master_password)?;
        let mut cmd = Command::new("bw");
        cmd.args(["list", "items"]);
        if let Some(ref s) = self.session_key {
            cmd.args(["--session", s]);
        }
        cmd.output()
            .map_err(|e| anyhow!("failed to run bw list items: {}", e))
    }

    fn bw_cmd_list_folders(
        &mut self,
        master_password: Option<&str>,
    ) -> Result<std::process::Output> {
        self.ensure_unlocked(master_password)?;
        let mut cmd = Command::new("bw");
        cmd.args(["list", "folders"]);
        if let Some(ref s) = self.session_key {
            cmd.args(["--session", s]);
        }
        cmd.output()
            .map_err(|e| anyhow!("failed to run bw list folders: {}", e))
    }

    /// List all items
    pub fn list_items(&mut self, master_password: Option<&str>) -> Result<Vec<BitwardenItem>> {
        let output = self.bw_cmd_list_items(master_password)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to get items: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            return Err(anyhow!(
                "Empty response from Bitwarden. Is the vault locked or empty?"
            ));
        }

        let trimmed = stdout.trim();
        if !trimmed.starts_with('[') && !trimmed.starts_with('{') {
            return Err(anyhow!(
                "Invalid response from Bitwarden (not JSON): {}. Is the vault locked?",
                trimmed.chars().take(200).collect::<String>()
            ));
        }

        let items: Vec<BitwardenItem> = serde_json::from_slice(&output.stdout).map_err(|e| {
            anyhow!(
                "Failed to parse items JSON: {}. Response: {}",
                e,
                trimmed.chars().take(500).collect::<String>()
            )
        })?;
        Ok(items)
    }

    /// List all folders
    pub fn list_folders(&mut self, master_password: Option<&str>) -> Result<Vec<BitwardenFolder>> {
        let output = self.bw_cmd_list_folders(master_password)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to get folders: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            return Err(anyhow!(
                "Empty response from Bitwarden. Is the vault locked or empty?"
            ));
        }

        let trimmed = stdout.trim();
        if !trimmed.starts_with('[') && !trimmed.starts_with('{') {
            return Err(anyhow!(
                "Invalid response from Bitwarden (not JSON): {}. Is the vault locked?",
                trimmed.chars().take(200).collect::<String>()
            ));
        }

        let folders: Vec<BitwardenFolder> =
            serde_json::from_slice(&output.stdout).map_err(|e| {
                anyhow!(
                    "Failed to parse folders JSON: {}. Response: {}",
                    e,
                    trimmed.chars().take(500).collect::<String>()
                )
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
        let items = self.list_items(master_password)?;
        let folders = self.list_folders(master_password)?;

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
                let folder_name = item
                    .folder_id
                    .as_str()
                    .and_then(|id| folder_map.get(id))
                    .map(|s| s.as_str())
                    .unwrap_or("");

                let matches_prefix = if let Some(prefix) = folder_prefix {
                    folder_name.starts_with(prefix)
                } else {
                    true
                };

                let matches_service = if let Some(service) = service_name {
                    item.name
                        .as_str()
                        .map(|n| n.to_lowercase())
                        .unwrap_or_default()
                        .contains(&service.to_lowercase())
                } else {
                    true
                };

                matches_prefix && matches_service
            })
            .collect();

        Ok(filtered)
    }
}
