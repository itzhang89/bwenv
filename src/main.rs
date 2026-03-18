use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};

mod bitwarden;
mod commands;
mod config;
mod parser;

use commands::{config_cmd, generate, list};
use config::Config;

#[derive(Parser)]
#[command(name = "bwenv")]
#[command(about = "Bitwarden to Environment Variables Tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Filter by prefix
    #[arg(short, long)]
    prefix: Option<String>,

    /// Specify services (can be specified multiple times)
    #[arg(short = 's', long)]
    service: Vec<String>,

    /// Config file path (one service per line)
    #[arg(short, long)]
    config: Option<String>,

    /// Output file path
    #[arg(short, long)]
    output: Option<String>,

    /// Output format: shell, env, json
    #[arg(short, long, default_value = "shell")]
    format: String,
}

#[derive(Subcommand)]
enum Commands {
    /// List Bitwarden vault items
    List {
        /// Filter by prefix
        #[arg(short, long)]
        prefix: Option<String>,

        /// Filter by service name
        #[arg(short, long)]
        service: Option<String>,

        /// List all folders
        #[arg(long)]
        folders: bool,
    },

    /// Generate environment variables
    Gen {
        /// Filter by prefix
        #[arg(short, long)]
        prefix: Option<String>,

        /// Specify services (can be specified multiple times)
        #[arg(short = 's', long)]
        service: Vec<String>,

        /// Config file path
        #[arg(short, long)]
        config: Option<String>,

        /// Output file path
        #[arg(short, long)]
        output: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "shell")]
        format: String,
    },

    /// Project management
    Project {
        #[command(subcommand)]
        command: Option<ProjectCommands>,
    },

    /// Show current project
    Current,

    /// Configuration management
    Config {
        #[command(subcommand)]
        command: Option<ConfigCommands>,
    },

    /// Use project and export environment variables
    Use {
        /// Project name (if not specified, interactive selection or auto-detect from .bwenv)
        name: Option<String>,

        /// Output file path
        #[arg(short, long)]
        output: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "shell")]
        format: String,
    },
}

#[derive(Subcommand)]
enum ProjectCommands {
    /// List all projects
    List,
    /// Add project: bwenv project add <projectname> <services> [prefix]
    Add {
        name: String,
        /// Service list (comma-separated, empty means all)
        services: String,
        /// Prefix (optional)
        prefix: Option<String>,
    },
    /// Load projects from file
    Load {
        path: String,
    },
    /// Remove project
    Remove {
        name: String,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Show configuration
    Show,
    /// Initialize
    Init,
    /// 设置
    Set {
        key: String,
        value: String,
    },
}

fn get_master_password() -> Result<Option<String>> {
    if let Ok(password) = std::env::var("BW_MASTER_PASSWORD") {
        if !password.is_empty() {
            return Ok(Some(password));
        }
    }

    if let Ok(config) = Config::load() {
        if let Some(password) = config.get_master_password() {
            return Ok(Some(password.to_string()));
        }
    }

    use dialoguer::Password;

    let password: String = Password::new()
        .with_prompt("请输入 Bitwarden 主密码")
        .interact()?;

    if password.is_empty() {
        Ok(None)
    } else {
        Ok(Some(password))
    }
}

fn run_generate(
    config: &Config,
    prefix: Option<String>,
    service: Vec<String>,
    config_file: Option<String>,
    output: Option<String>,
    format: String,
) -> Result<()> {
    let effective_prefix = prefix
        .or_else(|| config.get_current_project().map(|p| p.prefix.clone()));

    let services: Option<Vec<String>> = if !service.is_empty() {
        Some(service)
    } else if let Some(ref cf) = config_file {
        Some(config::load_services_from_file(cf)?)
    } else if let Some(project) = config.get_current_project() {
        // 如果项目配置了 services 则使用，否则查询全部
        project.services.clone()
    } else {
        // 没有指定任何服务，查询全部
        None
    };

    let master_password = get_master_password()?;
    let master_password_opt = master_password.as_deref();

    generate::generate_env(
        master_password_opt,
        effective_prefix.as_deref(),
        services,
        &format,
        output.as_deref(),
    )
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = Config::load()?;

    // 保存 cli 参数的副本供后续使用
    let cli_prefix = cli.prefix.clone();
    let cli_service = cli.service.clone();
    let cli_config = cli.config.clone();
    let cli_output = cli.output.clone();
    let cli_format = cli.format.clone();

    // 处理默认命令：bwenv 直接运行 generate
    let command = cli.command.unwrap_or(Commands::Gen {
        prefix: cli_prefix.clone(),
        service: cli_service.clone(),
        config: cli_config.clone(),
        output: cli_output.clone(),
        format: cli_format.clone(),
    });

    match command {
        Commands::List { prefix, service, folders } => {
            if folders {
                let master_password = get_master_password()?;
                let mut client = crate::bitwarden::client::BitwardenClient::new();
                let bw_folders = client.list_folders(master_password.as_deref())?;
                println!("Available folders:");
                for folder in &bw_folders {
                    let name = folder.name.as_str().unwrap_or("(unnamed)");
                    println!("  - {}", name);
                }
            } else {
                let effective_prefix = prefix
                    .or_else(|| config.get_current_project().map(|p| p.prefix.clone()));

                let master_password = get_master_password()?;
                list::list_items(master_password.as_deref(), effective_prefix.as_deref(), service.as_deref())?;
            }
        }

        Commands::Gen {
            prefix,
            service,
            config: config_file,
            output,
            format,
        } => {
            run_generate(&config, prefix, service, config_file, output, format)?;
        }

        Commands::Project { command } => {
            match command {
                Some(ProjectCommands::List) => {
                    config_cmd::list_projects(&config)?;
                }
                Some(ProjectCommands::Add { name, services, prefix }) => {
                    let services: Option<Vec<String>> = if services.is_empty() {
                        None
                    } else {
                        Some(
                            services
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect(),
                        )
                    };
                    let prefix = prefix.unwrap_or_default();
                    config.add_project(config::models::Project::new(&name, &prefix, services))?;
                    println!("Project added: {}", name);
                }
                Some(ProjectCommands::Load { path }) => {
                    let projects = Config::load_projects_from_file(&path)?;
                    let count = projects.len();
                    for project in projects {
                        if !config.projects.iter().any(|p| p.name == project.name) {
                            config.projects.push(project);
                        }
                    }
                    config.save()?;
                    println!("Loaded {} projects from file", count);
                }
                Some(ProjectCommands::Remove { name }) => {
                    config.remove_project(&name)?;
                    println!("Project removed: {}", name);
                }
                None => {
                    config_cmd::list_projects(&config)?;
                }
            }
        }

        Commands::Current => {
            if let Some(project) = config.get_current_project() {
                println!("Current project: {}", project.name);
                println!("Prefix: {}", project.prefix);
                match &project.services {
                    Some(svc) if !svc.is_empty() => println!("Services: {:?}", svc),
                    Some(_) | None => println!("Services: (all)"),
                }
            } else {
                println!("No project selected");
            }
        }

        Commands::Config { command } => {
            match command {
                Some(ConfigCommands::Show) => {
                    config_cmd::show_config(&config)?;
                }
                Some(ConfigCommands::Init) => {
                    config_cmd::init_config()?;
                }
                Some(ConfigCommands::Set { key, value }) => {
                    match key.as_str() {
                        "master_password" => {
                            config.set_master_password(value)?;
                            println!("master_password set");
                        }
                        "default_format" => {
                            config.set_default_format(value)?;
                            println!("default_format set");
                        }
                        _ => {
                            return Err(anyhow!("Unknown config key: {}", key));
                        }
                    }
                }
                None => {
                    config_cmd::show_config(&config)?;
                }
            }
        }

        Commands::Use { name, output, format } => {
            // If project name is specified, switch directly; otherwise auto-detect from .bwenv or interactive selection
            if let Some(project_name) = name {
                // 直接切换到指定项目
                config.set_current_project(&project_name)?;
                println!("Switched to project: {}", project_name);
            } else if let Ok(Some(project)) = config::Config::load_project_from_dir() {
                // 如果项目不存在于配置中，添加它
                if !config.projects.iter().any(|p| p.name == project.name) {
                    config.projects.push(project.clone());
                    config.save()?;
                }
                config.set_current_project(&project.name)?;
                println!("Auto-detected project: {} (from .bwenv)", project.name);
            } else {
                // 交互式选择项目
                use dialoguer::Select;
                let projects: Vec<&str> = config.projects.iter().map(|p| p.name.as_str()).collect();

                if projects.is_empty() {
                    return Err(anyhow!("未配置任何项目，请先使用 'bwenv project add' 添加项目"));
                }

                let current = config.current_project.clone().unwrap_or_default();
                let selection = Select::new()
                    .with_prompt("选择项目")
                    .items(&projects)
                    .default(projects.iter().position(|p| *p == current).unwrap_or(0))
                    .interact()?;

                let selected = projects[selection].to_string();
                config.set_current_project(&selected)?;
                println!("Switched to project: {}", selected);
            }

            // 只有指定了 --output 或其他参数时才生成环境变量
            // 否则只切换项目
            if output.is_some() || !cli_service.is_empty() || cli_prefix.is_some() || cli_config.is_some() {
                run_generate(&config, cli_prefix, cli_service, cli_config, output, format)?;
            }
        }
    }

    Ok(())
}
