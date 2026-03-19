use anyhow::Result;
use crate::config::Config;

pub fn show_config(config: &Config) -> Result<()> {
    println!("Current configuration:");
    println!();
    if let Some(ref bw) = config.bitwarden {
        if bw.master_password.is_some() {
            println!("  master_password: ********");
        } else {
            println!("  master_password: (not set, will prompt at runtime)");
        }
    } else {
        println!("  master_password: (not set, will prompt at runtime)");
    }
    println!("  default_format: {:?}", config.default_format);
    println!();
    println!("  projects: {}", config.projects.len());
    if let Some(ref current) = config.current_project {
        println!("  current_project: {}", current);
    } else {
        println!("  current_project: (not set)");
    }
    Ok(())
}

pub fn init_config() -> Result<()> {
    let default_config = r#"# Bitwarden Configuration
bitwarden:
  master_password: "your-master-password"

# Default output format (shell, env, json)
default_format: "shell"

# Projects
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
    println!("Config file created: {}", config_path.display());
    Ok(())
}

/// Output shell wrapper function for bwenv
pub fn shell_init(shell: Option<&str>) -> Result<()> {
    let shell = shell.unwrap_or("zsh");

    match shell {
        "zsh" => {
            println!("{}", ZSH_WRAPPER);
        }
        "bash" => {
            println!("{}", BASH_WRAPPER);
        }
        _ => {
            return Err(anyhow::anyhow!("Unsupported shell: {}. Use 'zsh' or 'bash'", shell));
        }
    }
    Ok(())
}

const ZSH_WRAPPER: &str = r#"# bwenv shell wrapper - add to your ~/.zshrc
# Run: echo 'source <(bwenv shell-init)' >> ~/.zshrc
#
# 'bwenv' (no args) - will eval output (auto-export env vars)
# 'bwenv <subcommand>' - works normally

bwenv() {
    if [ -z "$1" ]; then
        eval $(command bwenv)
    else
        command bwenv "$@"
    fi
}
"#;

const BASH_WRAPPER: &str = r#"# bwenv shell wrapper - add to your ~/.bashrc
# Run: echo 'source <(bwenv shell-init bash)' >> ~/.bashrc
#
# 'bwenv' (no args) - will eval output (auto-export env vars)
# 'bwenv <subcommand>' - works normally

bwenv() {
    if [ -z "$1" ]; then
        eval $(command bwenv)
    else
        command bwenv "$@"
    fi
}
"#;

pub fn list_projects(config: &Config) -> Result<()> {
    if config.projects.is_empty() {
        println!("No projects. Use 'bwenv project add' to add one");
        return Ok(());
    }
    println!("Projects:\n");
    for (i, project) in config.projects.iter().enumerate() {
        let marker = if config.current_project.as_deref() == Some(&project.name) {
            "*"
        } else {
            " "
        };
        let prefix_display = if project.prefix.is_empty() { "(none)" } else { &project.prefix };
        let services_display = match &project.services {
            Some(svc) if !svc.is_empty() => format!("{:?}", svc),
            Some(_) | None => "all".to_string(),
        };
        println!("{} {}. {} (prefix: {}, services: {})",
            marker, i + 1, project.name, prefix_display, services_display);
    }
    Ok(())
}
