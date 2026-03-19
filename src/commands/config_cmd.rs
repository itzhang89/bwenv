use crate::config::Config;
use anyhow::{anyhow, Result};

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
    let detected_shell = shell.map(|s| s.to_string()).unwrap_or_else(detect_shell);

    let wrapper = match detected_shell.as_str() {
        "zsh" => ZSH_WRAPPER,
        "bash" => BASH_WRAPPER,
        s => return Err(anyhow!("Unsupported shell: {}. Use 'zsh' or 'bash'", s)),
    };

    add_to_shell_config(&detected_shell)?;

    println!("{}", wrapper);

    Ok(())
}

/// Auto-detect current shell type
fn detect_shell() -> String {
    // Check $SHELL first
    if let Ok(shell_path) = std::env::var("SHELL") {
        let shell = std::path::Path::new(&shell_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if shell == "zsh" || shell == "bash" {
            return shell.to_string();
        }
    }

    // Check parent process name
    if let Ok(ppid) = std::env::var("PPID") {
        if let Ok(pid) = ppid.parse::<u32>() {
            if let Ok(path) = std::fs::read_link(format!("/proc/{}/exe", pid)) {
                let exe_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if exe_name.contains("zsh") {
                    return "zsh".to_string();
                } else if exe_name.contains("bash") {
                    return "bash".to_string();
                }
            }
        }
    }

    // Default to zsh on macOS, bash on Linux
    #[cfg(target_os = "macos")]
    {
        "zsh".to_string()
    }
    #[cfg(not(target_os = "macos"))]
    {
        "bash".to_string()
    }
}

/// Add bwenv wrapper to shell config file
fn add_to_shell_config(shell: &str) -> Result<()> {
    let config_path = match shell {
        "zsh" => dirs::home_dir().map(|p| p.join(".zshrc")),
        "bash" => {
            // Try .bashrc first, then .bash_profile
            if let Some(home) = dirs::home_dir() {
                let bashrc = home.join(".bashrc");
                if bashrc.exists() {
                    Some(bashrc)
                } else {
                    Some(home.join(".bash_profile"))
                }
            } else {
                None
            }
        }
        _ => None,
    };

    let config_path = config_path.ok_or_else(|| anyhow!("Cannot find home directory"))?;

    // Check if already added
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        if content.contains("bwenv") && content.contains("shell-init") {
            println!("bwenv wrapper already found in {}", config_path.display());
            return Ok(());
        }
    }

    // Generate the wrapper
    let wrapper = match shell {
        "zsh" => ZSH_WRAPPER,
        "bash" => BASH_WRAPPER,
        _ => unreachable!(),
    };

    // Append to config file
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config_path)?;

    use std::io::Write;
    writeln!(file, "\n{}", wrapper)?;

    println!("Added bwenv wrapper to {}", config_path.display());
    println!(
        "Restart your shell or run: source {}",
        config_path.display()
    );

    Ok(())
}

const ZSH_WRAPPER: &str = r#"# bwenv shell wrapper - add to your ~/.zshrc
# Run: echo 'source <(bwenv config shell-init)' >> ~/.zshrc
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
# Run: echo 'source <(bwenv config shell-init bash)' >> ~/.bashrc
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
        let prefix_display = if project.prefix.is_empty() {
            "(none)"
        } else {
            &project.prefix
        };
        let services_display = match &project.services {
            Some(svc) if !svc.is_empty() => format!("{:?}", svc),
            Some(_) | None => "all".to_string(),
        };
        println!(
            "{} {}. {} (prefix: {}, services: {})",
            marker,
            i + 1,
            project.name,
            prefix_display,
            services_display
        );
    }
    Ok(())
}
