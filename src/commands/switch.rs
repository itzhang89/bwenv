use anyhow::{anyhow, Result};
use dialoguer::Select;
use crate::config::Config;

/// 切换当前项目
pub fn switch_command(project_name: Option<String>, config: &mut Config) -> Result<()> {
    match project_name {
        Some(name) => {
            // 直接切换到指定项目
            config.set_current_project(&name)?;
            println!("已切换到项目: {}", name);
        }
        None => {
            // 交互式选择
            let projects: Vec<&str> = config.projects.iter().map(|p| p.name.as_str()).collect();

            if projects.is_empty() {
                return Err(anyhow!("未配置任何项目，请先在配置文件中添加项目"));
            }

            // 显示当前选中状态
            let current = config.current_project.clone().unwrap_or_default();
            let selection = Select::new()
                .with_prompt("选择项目")
                .items(&projects)
                .default(projects.iter().position(|p| *p == current).unwrap_or(0))
                .interact()?;

            let selected = projects[selection].to_string();
            config.set_current_project(&selected)?;
            println!("已切换到项目: {}", selected);
        }
    }
    Ok(())
}

/// 列出已配置的项目
pub fn list_projects(config: &Config) -> Result<()> {
    if config.projects.is_empty() {
        println!("未配置任何项目");
        return Ok(());
    }

    println!("已配置的项目:\n");
    for (i, project) in config.projects.iter().enumerate() {
        let marker = if config.current_project.as_deref() == Some(&project.name) {
            "*"
        } else {
            " "
        };
        println!("{} {}. {} (前缀: {}, 服务: {:?})", marker, i + 1, project.name, project.prefix, project.services);
    }

    Ok(())
}
