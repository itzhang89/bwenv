use anyhow::{anyhow, Result};
use clap::Parser;
use crate::config::Config;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub enum ConfigCommands {
    /// 设置配置项
    Set {
        /// 配置项名称
        key: String,
        /// 配置项值
        value: String,
    },
    /// 显示当前配置
    Show,
    /// 初始化配置文件
    Init,
}

pub fn config_command(cmd: ConfigCommands, config: &mut Config) -> Result<()> {
    match cmd {
        ConfigCommands::Set { key, value } => {
            match key.as_str() {
                "master_password" => {
                    config.set_master_password(value)?;
                    println!("已设置 master_password");
                }
                "default_prefix" => {
                    config.default_prefix = Some(value);
                    config.save()?;
                    println!("已设置 default_prefix");
                }
                "default_format" => {
                    config.default_format = Some(value);
                    config.save()?;
                    println!("已设置 default_format");
                }
                _ => {
                    return Err(anyhow!("未知的配置项: {}", key));
                }
            }
        }
        ConfigCommands::Show => {
            println!("当前配置:");
            println!();
            if let Some(ref bw) = config.bitwarden {
                if bw.master_password.is_some() {
                    println!("  master_password: ********");
                } else {
                    println!("  master_password: (未设置，将在运行时提示输入)");
                }
            } else {
                println!("  master_password: (未设置，将在运行时提示输入)");
            }
            println!("  default_prefix: {:?}", config.default_prefix);
            println!("  default_format: {:?}", config.default_format);
            println!("  default_services: {:?}", config.default_services);
            println!("  projects: {} 个项目", config.projects.len());
            if let Some(ref current) = config.current_project {
                println!("  current_project: {}", current);
            }
        }
        ConfigCommands::Init => {
            // 创建默认配置文件
            let default_config = r#"# Bitwarden API Key 配置
bitwarden:
  api_key: "your-api-key"
  api_secret: "your-api-secret"

# 默认前缀
default_prefix: "dev"

# 默认输出格式 (shell, env, json)
default_format: "shell"

# 默认服务列表
default_services:
  - mysql
  - redis
  - github

# 项目/目录配置
projects:
  - name: "dev/project1"
    prefix: "dev"
    services:
      - mysql
      - redis
  - name: "prod/api"
    prefix: "prod"
    services:
      - mysql
      - postgres

# 当前选中的项目（由 switch 命令自动更新）
# current_project: "dev/project1"
"#;

            let config_path = dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".bwenv")
                .join("config.yaml");

            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::write(&config_path, default_config)?;
            println!("配置文件已创建: {}", config_path.display());
            println!("请编辑配置文件并设置您的 API Key");
        }
    }

    Ok(())
}
