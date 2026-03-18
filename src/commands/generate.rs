use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;
use crate::bitwarden::client::BitwardenClient;
use crate::parser::env_gen::{to_env_format, to_json_format, to_shell_format, item_to_env_vars, EnvVar};

/// 查找项目目录下的 .claude/settings.local.json
fn find_claude_settings_path() -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    let settings_path = current_dir.join(".claude").join("settings.local.json");
    Some(settings_path)
}

/// 读取或创建 Claude 设置
fn read_claude_settings() -> Result<serde_json::Map<String, serde_json::Value>> {
    let settings_path = find_claude_settings_path()
        .ok_or_else(|| anyhow!("无法获取当前目录"))?;

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

/// 写入 Claude 设置
fn write_claude_settings(settings: &serde_json::Map<String, serde_json::Value>) -> Result<()> {
    let settings_path = find_claude_settings_path()
        .ok_or_else(|| anyhow!("无法获取当前目录"))?;

    // 确保 .claude 目录存在
    if let Some(parent) = settings_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    let json_str = serde_json::to_string_pretty(settings)?;
    fs::write(&settings_path, json_str)?;

    Ok(())
}

/// 获取当前项目名称
fn get_current_project_name() -> Option<String> {
    if let Ok(config) = crate::config::Config::load() {
        if let Some(project) = config.get_current_project() {
            return Some(project.name.clone());
        }
    }
    None
}

/// 将环境变量写入 Claude Code 项目配置（合并模式）
fn add_to_claude_settings(env_vars: &[EnvVar], project_name: &str) -> Result<()> {
    let mut settings = read_claude_settings()?;

    // 获取或创建 env 对象
    let env = settings.entry("env").or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

    let env_map = env.as_object_mut()
        .ok_or_else(|| anyhow!("env 字段格式错误"))?;

    // 添加新的环境变量
    for var in env_vars {
        env_map.insert(var.key.clone(), serde_json::Value::String(var.value.clone()));
    }

    // 更新或创建元数据
    let metadata = settings.entry("_bwenv").or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    let metadata_map = metadata.as_object_mut()
        .ok_or_else(|| anyhow!("_bwenv 字段格式错误"))?;

    // 获取当前项目的 var 列表
    let project_vars = metadata_map.entry(project_name).or_insert_with(|| serde_json::Value::Array(vec![]));
    let vars_array = project_vars.as_array_mut()
        .ok_or_else(|| anyhow!("项目 var 列表格式错误"))?;

    // 添加新 var 到列表（避免重复）
    for var in env_vars {
        if !vars_array.iter().any(|v| v.as_str() == Some(&var.key)) {
            vars_array.push(serde_json::Value::String(var.key.clone()));
        }
    }

    write_claude_settings(&settings)?;

    Ok(())
}

/// 从 Claude Code 项目配置中移除当前项目的环境变量
fn remove_from_claude_settings(project_name: &str) -> Result<usize> {
    let settings_path = find_claude_settings_path()
        .ok_or_else(|| anyhow!("无法获取当前目录"))?;

    if !settings_path.exists() {
        return Ok(0);
    }

    let content = fs::read_to_string(&settings_path)?;
    let mut settings: serde_json::Value = serde_json::from_str(&content)?;

    let settings_map = match &mut settings {
        serde_json::Value::Object(map) => map,
        _ => return Ok(0),
    };

    // 获取要移除的 var 列表
    let vars_to_remove: Vec<String> = if let Some(metadata) = settings_map.get("_bwenv") {
        if let serde_json::Value::Object(metadata_map) = metadata {
            if let Some(project_vars) = metadata_map.get(project_name) {
                if let serde_json::Value::Array(arr) = project_vars {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    // 移除环境变量
    let mut removed_count = 0;
    if let Some(env) = settings_map.get_mut("env") {
        if let serde_json::Value::Object(env_map) = env {
            for key in &vars_to_remove {
                if env_map.remove(key).is_some() {
                    removed_count += 1;
                }
            }
        }
    }

    // 清理元数据
    if let Some(metadata) = settings_map.get_mut("_bwenv") {
        if let serde_json::Value::Object(metadata_map) = metadata {
            if let Some(project_vars) = metadata_map.get_mut(project_name) {
                *project_vars = serde_json::Value::Array(vec![]);
            }
        }
    }

    let json_str = serde_json::to_string_pretty(&settings)?;
    fs::write(&settings_path, json_str)?;

    Ok(removed_count)
}

/// 生成环境变量
/// services 为 None 或空时，查询全部（只按 prefix 过滤）
pub fn generate_env(
    master_password: Option<&str>,
    prefix: Option<&str>,
    services: Option<Vec<String>>,
    format: &str,
    output_path: Option<&str>,
) -> Result<()> {
    let mut client = BitwardenClient::new();

    // 收集所有服务的环境变量
    let mut all_vars = Vec::new();

    // 如果指定了 services，则按服务名筛选；否则查询全部
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
            // services 为空 Vec，等同于查询全部
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
        // services 为 None，查询全部
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
        println!("未找到匹配的 items");
        return Ok(());
    }

    // 按格式输出
    let output_content = match format {
        "env" => to_env_format(&all_vars),
        "json" => to_json_format(&all_vars),
        _ => to_shell_format(&all_vars),
    };

    if let Some(path) = output_path {
        match path {
            "claude" => {
                // 获取当前项目名称
                let project_name = get_current_project_name()
                    .unwrap_or_else(|| "default".to_string());

                // 添加到 Claude Code 项目配置
                add_to_claude_settings(&all_vars, &project_name)?;

                let settings_path = find_claude_settings_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| ".claude/settings.local.json".to_string());

                println!("✓ 环境变量已添加到 Claude Code 项目配置");
                println!();
                println!("  项目: {}", project_name);
                println!("  配置文件: {}", settings_path);
                println!();
                println!("已添加的环境变量 ({} 个):", all_vars.len());
                for var in &all_vars {
                    println!("  + {}", var.key);
                }
                println!();
                println!("⚠ 请重启 Claude Code 使配置生效");
            }
            "claude:remove" | "claude:clear" => {
                // 移除当前项目的环境变量
                let project_name = get_current_project_name()
                    .unwrap_or_else(|| "default".to_string());

                let removed = remove_from_claude_settings(&project_name)?;

                let settings_path = find_claude_settings_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| ".claude/settings.local.json".to_string());

                println!("✓ 已从 Claude Code 项目配置中移除环境变量");
                println!();
                println!("  项目: {}", project_name);
                println!("  配置文件: {}", settings_path);
                println!();
                println!("已移除的环境变量 ({} 个)", removed);
                println!();
                println!("⚠ 请重启 Claude Code 使配置生效");
            }
            _ => {
                fs::write(path, &output_content)?;
                println!("已导出到: {}", path);
            }
        }
    } else {
        println!("{}", output_content);
    }

    Ok(())
}
