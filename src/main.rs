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

    /// 指定前缀筛选
    #[arg(short, long)]
    prefix: Option<String>,

    /// 指定服务名（可多次指定）
    #[arg(short = 's', long)]
    service: Vec<String>,

    /// 配置文件路径（每行一个服务名）
    #[arg(short, long)]
    config: Option<String>,

    /// 输出文件路径
    #[arg(short, long)]
    output: Option<String>,

    /// 输出格式：shell, env, json
    #[arg(short, long, default_value = "shell")]
    format: String,
}

#[derive(Subcommand)]
enum Commands {
    /// 列出 Bitwarden vault 中的 items
    List {
        /// 指定前缀筛选
        #[arg(short, long)]
        prefix: Option<String>,

        /// 指定服务名筛选
        #[arg(short, long)]
        service: Option<String>,

        /// 列出所有文件夹
        #[arg(long)]
        folders: bool,
    },

    /// 生成环境变量
    Gen {
        /// 指定前缀筛选
        #[arg(short, long)]
        prefix: Option<String>,

        /// 指定服务名（可多次指定）
        #[arg(short = 's', long)]
        service: Vec<String>,

        /// 配置文件路径
        #[arg(short, long)]
        config: Option<String>,

        /// 输出文件路径
        #[arg(short, long)]
        output: Option<String>,

        /// 输出格式
        #[arg(short, long, default_value = "shell")]
        format: String,
    },

    /// 项目管理
    Project {
        #[command(subcommand)]
        command: Option<ProjectCommands>,
    },

    /// 查看当前项目
    Current,

    /// 配置管理
    Config {
        #[command(subcommand)]
        command: Option<ConfigCommands>,
    },

    /// 使用项目并导出环境变量
    Use {
        /// 项目名称（不指定则交互式选择或从当前目录 .bwenv 自动检测）
        name: Option<String>,

        /// 输出文件路径
        #[arg(short, long)]
        output: Option<String>,

        /// 输出格式
        #[arg(short, long, default_value = "shell")]
        format: String,
    },
}

#[derive(Subcommand)]
enum ProjectCommands {
    /// 列出所有项目
    List,
    /// 添加项目：bwenv project add <projectname> <services> [prefix]
    Add {
        name: String,
        /// 服务列表（逗号分隔，为空时查询全部）
        services: String,
        /// 前缀（可选）
        prefix: Option<String>,
    },
    /// 从文件加载项目
    Load {
        path: String,
    },
    /// 删除项目
    Remove {
        name: String,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// 显示配置
    Show,
    /// 初始化
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
                // 列出所有文件夹
                let master_password = get_master_password()?;
                let mut client = crate::bitwarden::client::BitwardenClient::new();
                let bw_folders = client.list_folders(master_password.as_deref())?;
                println!("可用文件夹:");
                for folder in &bw_folders {
                    let name = folder.name.as_str().unwrap_or("(无名称)");
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
                    println!("已添加项目: {}", name);
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
                    println!("已从文件加载 {} 个项目", count);
                }
                Some(ProjectCommands::Remove { name }) => {
                    config.remove_project(&name)?;
                    println!("已删除项目: {}", name);
                }
                None => {
                    config_cmd::list_projects(&config)?;
                }
            }
        }

        Commands::Current => {
            if let Some(project) = config.get_current_project() {
                println!("当前项目: {}", project.name);
                println!("前缀: {}", project.prefix);
                match &project.services {
                    Some(svc) if !svc.is_empty() => println!("服务: {:?}", svc),
                    Some(_) | None => println!("服务: (查询全部)"),
                }
            } else {
                println!("未选择项目");
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
                            println!("已设置 master_password");
                        }
                        "default_format" => {
                            config.set_default_format(value)?;
                            println!("已设置 default_format");
                        }
                        _ => {
                            return Err(anyhow!("未知的配置项: {}", key));
                        }
                    }
                }
                None => {
                    config_cmd::show_config(&config)?;
                }
            }
        }

        Commands::Use { name, output, format } => {
            // 如果指定了项目名，直接切换；否则从当前目录 .bwenv 自动检测或交互式选择
            if let Some(project_name) = name {
                // 直接切换到指定项目
                config.set_current_project(&project_name)?;
                println!("已切换到项目: {}", project_name);
            } else if let Ok(Some(project)) = config::Config::load_project_from_dir() {
                // 如果项目不存在于配置中，添加它
                if !config.projects.iter().any(|p| p.name == project.name) {
                    config.projects.push(project.clone());
                    config.save()?;
                }
                config.set_current_project(&project.name)?;
                println!("自动检测到项目: {} (来自 .bwenv)", project.name);
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
                println!("已切换到项目: {}", selected);
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
