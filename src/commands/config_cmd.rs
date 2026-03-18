use anyhow::Result;
use crate::config::Config;

pub fn show_config(config: &Config) -> Result<()> {
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
    println!("  default_format: {:?}", config.default_format);
    println!();
    println!("  项目: {} 个", config.projects.len());
    if let Some(ref current) = config.current_project {
        println!("  当前项目: {}", current);
    } else {
        println!("  当前项目: (未选择)");
    }
    Ok(())
}

pub fn init_config() -> Result<()> {
    let default_config = r#"# Bitwarden 配置
bitwarden:
  master_password: "your-master-password"

# 默认输出格式 (shell, env, json)
default_format: "shell"

# 项目配置
# projects:
#   - name: "dev"
#     prefix: "dev"
#     services:
#       - mysql
#       - redis
#       - github
"#;

    let config_path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".bwenv")
        .join("config.yaml");

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&config_path, default_config)?;
    println!("配置文件已创建: {}", config_path.display());
    Ok(())
}

pub fn list_projects(config: &Config) -> Result<()> {
    if config.projects.is_empty() {
        println!("暂无项目，请使用 'bwenv project add' 添加");
        return Ok(());
    }
    println!("项目列表:\n");
    for (i, project) in config.projects.iter().enumerate() {
        let marker = if config.current_project.as_deref() == Some(&project.name) {
            "*"
        } else {
            " "
        };
        let prefix_display = if project.prefix.is_empty() { "(无)" } else { &project.prefix };
        println!("{} {}. {} (前缀: {}, 服务: {:?})",
            marker, i + 1, project.name, prefix_display, project.services);
    }
    Ok(())
}
