use crate::bitwarden::models::{BitwardenItem, Login};
use crate::config::rules::generate_env_key;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub key: String,
    pub value: String,
}

/// Generate environment variables from Bitwarden item
pub fn item_to_env_vars(item: &BitwardenItem) -> Vec<EnvVar> {
    let mut vars = Vec::new();

    // Extract service name from item name (remove prefix)
    let service_name = extract_service_name(item.get_name().unwrap_or("unknown"));

    // Handle login type
    if let Some(login_val) = item.login.as_object() {
        let login = Login {
            username: login_val.get("username").cloned().unwrap_or(serde_json::Value::Null),
            password: login_val.get("password").cloned().unwrap_or(serde_json::Value::Null),
            uris: login_val.get("uris").cloned().unwrap_or(serde_json::Value::Null),
            totp: login_val.get("totp").cloned().unwrap_or(serde_json::Value::Null),
            fido2_credentials: login_val.get("fido2Credentials").cloned().unwrap_or(serde_json::Value::Null),
            password_revision_date: login_val.get("passwordRevisionDate").cloned().unwrap_or(serde_json::Value::Null),
        };

        if let Some(username) = login.get_username() {
            let key = generate_env_key(&service_name, "user");
            vars.push(EnvVar {
                key,
                value: username.to_string(),
            });
        }

        if let Some(password) = login.get_password() {
            let key = generate_env_key(&service_name, "password");
            vars.push(EnvVar {
                key,
                value: password.to_string(),
            });
        }

        // Handle URIs (take first)
        if let Some(uri) = login.get_uri() {
            let key = generate_env_key(&service_name, "url");
            vars.push(EnvVar {
                key,
                value: uri,
            });
        }

        if let Some(totp) = login.totp.as_str() {
            let key = generate_env_key(&service_name, "totp");
            vars.push(EnvVar {
                key,
                value: totp.to_string(),
            });
        }
    }

    // Handle custom fields
    if let Some(fields_arr) = item.fields.as_array() {
        for field_val in fields_arr {
            if let Some(field_obj) = field_val.as_object() {
                let field_name = field_obj.get("name").and_then(|v| v.as_str()).unwrap_or("field");
                let field_value = field_obj.get("value").and_then(|v| v.as_str()).unwrap_or("");
                let key = generate_env_key(&service_name, field_name);
                vars.push(EnvVar {
                    key,
                    value: field_value.to_string(),
                });
            }
        }
    }

    vars
}

/// Extract service name from item name
fn extract_service_name(item_name: &str) -> String {
    // Try to extract last part from path as service name
    // Example: "dev/mysql" -> "mysql", "prod/github/api_key" -> "github"
    let parts: Vec<&str> = item_name.split('/').collect();
    let last_part = parts.last().unwrap_or(&item_name);

    // Clean service name: lowercase, remove special chars
    last_part
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

/// Generate shell format output
pub fn to_shell_format(vars: &[EnvVar]) -> String {
    vars.iter()
        .map(|var| format!("export {}={}", var.key, shell_escape(&var.value)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Generate .env format output
pub fn to_env_format(vars: &[EnvVar]) -> String {
    vars.iter()
        .map(|var| format!("{}={}", var.key, var.value))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Generate JSON format output
pub fn to_json_format(vars: &[EnvVar]) -> String {
    let map: HashMap<&str, &str> = vars
        .iter()
        .map(|var| (var.key.as_str(), var.value.as_str()))
        .collect();
    serde_json::to_string_pretty(&map).unwrap_or_else(|_| "{}".to_string())
}

/// Shell escape
fn shell_escape(s: &str) -> String {
    if s.is_empty() {
        return "\"\"".to_string();
    }

    // If contains special characters, needs quotes
    if s.chars().any(|c| c.is_whitespace() || c == '"' || c == '$' || c == '`') {
        // Simple escape: replace " with \"
        let escaped = s.replace('"', "\\\"");
        format!("\"{}\"", escaped)
    } else {
        s.to_string()
    }
}
