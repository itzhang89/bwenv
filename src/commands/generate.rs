use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;
use crate::bitwarden::client::BitwardenClient;
use crate::parser::env_gen::{to_env_format, to_json_format, to_shell_format, item_to_env_vars, EnvVar};

/// Find .claude/settings.local.json in project directory
fn find_claude_settings_path() -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    let settings_path = current_dir.join(".claude").join("settings.local.json");
    Some(settings_path)
}

/// Read or create Claude settings
fn read_claude_settings() -> Result<serde_json::Map<String, serde_json::Value>> {
    let settings_path = find_claude_settings_path()
        .ok_or_else(|| anyhow!("Cannot get current directory"))?;

    if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        let settings: serde_json::Value = serde_json::from_str(&content)?;
        if let serde_json::Value::Object(map) = settings {
            Ok(map)
        } else {
            Ok(serde_json::Map::new())
        }
    } else {
        Ok(serde_json::Map::new())
    }
}

/// Write Claude settings
fn write_claude_settings(settings: &serde_json::Map<String, serde_json::Value>) -> Result<()> {
    let settings_path = find_claude_settings_path()
        .ok_or_else(|| anyhow!("Cannot get current directory"))?;

    // Ensure .claude directory exists
    if let Some(parent) = settings_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    let json_str = serde_json::to_string_pretty(settings)?;
    fs::write(&settings_path, json_str)?;

    Ok(())
}

/// Get current project name
fn get_current_project_name() -> Option<String> {
    if let Ok(config) = crate::config::Config::load() {
        if let Some(project) = config.get_current_project() {
            return Some(project.name.clone());
        }
    }
    None
}

/// Add env vars to Claude Code project config (merge mode)
fn add_to_claude_settings(env_vars: &[EnvVar], project_name: &str) -> Result<()> {
    let mut settings = read_claude_settings()?;

    // Get or create env object
    let env = settings.entry("env").or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

    let env_map = env.as_object_mut()
        .ok_or_else(|| anyhow!("Invalid env field format"))?;

    // Add new environment variables
    for var in env_vars {
        env_map.insert(var.key.clone(), serde_json::Value::String(var.value.clone()));
    }

    // Update or create metadata
    let metadata = settings.entry("_bwenv").or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    let metadata_map = metadata.as_object_mut()
        .ok_or_else(|| anyhow!("Invalid _bwenv field format"))?;

    // Get current project var list
    let project_vars = metadata_map.entry(project_name).or_insert_with(|| serde_json::Value::Array(vec![]));
    let vars_array = project_vars.as_array_mut()
        .ok_or_else(|| anyhow!("Invalid project var list format"))?;

    // Add new var to list (avoid duplicates)
    for var in env_vars {
        if !vars_array.iter().any(|v| v.as_str() == Some(&var.key)) {
            vars_array.push(serde_json::Value::String(var.key.clone()));
        }
    }

    write_claude_settings(&settings)?;

    Ok(())
}

/// Remove env vars from Claude Code project config
fn remove_from_claude_settings(project_name: &str) -> Result<usize> {
    let settings_path = find_claude_settings_path()
        .ok_or_else(|| anyhow!("Cannot get current directory"))?;

    if !settings_path.exists() {
        return Ok(0);
    }

    let content = fs::read_to_string(&settings_path)?;
    let mut settings: serde_json::Value = serde_json::from_str(&content)?;

    let settings_map = match &mut settings {
        serde_json::Value::Object(map) => map,
        _ => return Ok(0),
    };

    // Get var list to remove
    let vars_to_remove: Vec<String> = if let Some(serde_json::Value::Object(metadata_map)) = settings_map.get("_bwenv") {
        if let Some(serde_json::Value::Array(arr)) = metadata_map.get(project_name) {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    // Remove environment variables
    let mut removed_count = 0;
    if let Some(serde_json::Value::Object(env_map)) = settings_map.get_mut("env") {
        for key in &vars_to_remove {
            if env_map.remove(key).is_some() {
                removed_count += 1;
            }
        }
    }

    // Clean up metadata: delete current project record and remove all empty arrays
    if let Some(serde_json::Value::Object(metadata_map)) = settings_map.get_mut("_bwenv") {
            // Delete current project
            metadata_map.remove(project_name);

            // Remove all keys with empty arrays
            let empty_keys: Vec<String> = metadata_map
                .iter()
                .filter(|(_, v)| {
                    if let serde_json::Value::Array(arr) = v {
                        arr.is_empty()
                    } else {
                        false
                    }
                })
                .map(|(k, _)| k.clone())
                .collect();

            for key in empty_keys {
                metadata_map.remove(&key);
            }

            // If _bwenv becomes empty object, delete the entire field
            if metadata_map.is_empty() {
                settings_map.remove("_bwenv");
            }
    }

    let json_str = serde_json::to_string_pretty(&settings)?;
    fs::write(&settings_path, json_str)?;

    Ok(removed_count)
}

/// Generate environment variables
/// If services is None or empty, query all (only filter by prefix)
pub fn generate_env(
    master_password: Option<&str>,
    prefix: Option<&str>,
    services: Option<Vec<String>>,
    format: &str,
    output_path: Option<&str>,
) -> Result<()> {
    let mut client = BitwardenClient::new();

    // Collect environment variables from all services
    let mut all_vars = Vec::new();

    // If services specified, filter by service name; otherwise query all
    if let Some(svc_list) = services {
        if !svc_list.is_empty() {
            for service in &svc_list {
                let items = client.list_items_by_folder_and_service(
                    master_password,
                    prefix,
                    Some(service),
                )?;

                for item in &items {
                    let vars = item_to_env_vars(item);
                    all_vars.extend(vars);
                }
            }
        } else {
            // Empty services Vec, same as querying all
            let items = client.list_items_by_folder_and_service(
                master_password,
                prefix,
                None,
            )?;

            for item in &items {
                let vars = item_to_env_vars(item);
                all_vars.extend(vars);
            }
        }
    } else {
        // services is None, query all
        let items = client.list_items_by_folder_and_service(
            master_password,
            prefix,
            None,
        )?;

        for item in &items {
            let vars = item_to_env_vars(item);
            all_vars.extend(vars);
        }
    }

    if all_vars.is_empty() {
        println!("No matching items found");
        return Ok(());
    }

    // Generate output in specified format
    let output_content = match format {
        "env" => to_env_format(&all_vars),
        "json" => to_json_format(&all_vars),
        _ => to_shell_format(&all_vars),
    };

    if let Some(path) = output_path {
        match path {
            "claude" => {
                // Get current project name
                let project_name = get_current_project_name()
                    .unwrap_or_else(|| "default".to_string());

                // Add to Claude Code project config
                add_to_claude_settings(&all_vars, &project_name)?;

                let settings_path = find_claude_settings_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| ".claude/settings.local.json".to_string());

                println!("✓ Environment variables added to Claude Code project config");
                println!();
                println!("  project: {}", project_name);
                println!("  config file: {}", settings_path);
                println!();
                println!("Added environment variables ({}):", all_vars.len());
                for var in &all_vars {
                    println!("  + {}", var.key);
                }
                println!();
                println!("⚠ Restart Claude Code to apply changes");
            }
            "claude:remove" | "claude:clear" => {
                // Remove current project environment variables
                let project_name = get_current_project_name()
                    .unwrap_or_else(|| "default".to_string());

                let removed = remove_from_claude_settings(&project_name)?;

                let settings_path = find_claude_settings_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| ".claude/settings.local.json".to_string());

                println!("✓ Environment variables removed from Claude Code project config");
                println!();
                println!("  project: {}", project_name);
                println!("  config file: {}", settings_path);
                println!();
                println!("Removed environment variables: {}", removed);
                println!();
                println!("⚠ Restart Claude Code to apply changes");
            }
            _ => {
                fs::write(path, &output_content)?;
                println!("Exported to: {}", path);
            }
        }
    } else {
        println!("{}", output_content);
    }

    Ok(())
}
