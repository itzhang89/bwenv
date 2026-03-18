use anyhow::{anyhow, Result};
use dialoguer::Select;
use crate::config::Config;

/// Switch current project
pub fn switch_command(project_name: Option<String>, config: &mut Config) -> Result<()> {
    match project_name {
        Some(name) => {
            config.set_current_project(&name)?;
            println!("Switched to project: {}", name);
        }
        None => {
            let projects: Vec<&str> = config.projects.iter().map(|p| p.name.as_str()).collect();

            if projects.is_empty() {
                return Err(anyhow!("No projects configured. Add a project first using 'bwenv project add'"));
            }

            let current = config.current_project.clone().unwrap_or_default();
            let selection = Select::new()
                .with_prompt("Select project")
                .items(&projects)
                .default(projects.iter().position(|p| *p == current).unwrap_or(0))
                .interact()?;

            let selected = projects[selection].to_string();
            config.set_current_project(&selected)?;
            println!("Switched to project: {}", selected);
        }
    }
    Ok(())
}
