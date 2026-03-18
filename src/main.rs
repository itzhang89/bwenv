use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use dialoguer::Input;

mod bitwarden;
mod commands;
mod config;
mod parser;

use commands::{config_cmd, generate, list, switch};
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

    /// 交互式选择服务
    #[arg(long)]
    select: bool,

    /// 指定项目名（覆盖当前项目）
    #[arg(long)]
    project: Option<String>,

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

        /// 指定项目名
        #[arg(long)]
        project: Option<String>,
    },

    /// 生成环境变量
    Gen {
        /// 指定前缀筛选
        #[arg(short, long)]
        prefix: Option<String>,

        /// 指定服务名（可多次指定）
        #[arg(short = 's', long)]
        service: Vec<String>,

        /// 交互式选择服务
        #[arg(long)]
        select: bool,

        /// 指定项目名
        #[arg(long)]
        project: Option<String>,

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

    /// 切换当前项目
    Switch {
        /// 项目名称
        name: Option<String>,

        /// 列出项目
        #[arg(long)]
        list: bool,
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
        /// 项目名称（必需）
        name: String,

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
    /// 添加项目
    Add {
        name: String,
        #[arg(short, long)]
        prefix: Option<String>,
        services: String,
    },
    /// 从文件加载项目
    Load {
        path: String,
    },
    /// 删除项目
    Remove {
        name: String,
    },
    /// 设置当前项目
    Use {
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

fn select_services(available_services: &[String]) -> Result<Vec<String>> {
    use dialoguer::MultiSelect;

    if available_services.is_empty() {
        return Err(anyhow!("没有可用的服务"));
    }

    let selection = MultiSelect::new()
        .with_prompt("选择服务（空格键选择，回车确认）")
        .items(available_services)
        .interact()?;

    Ok(selection.iter().map(|&i| available_services[i].clone()).collect())
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

    let password: String = Input::new()
        .with_prompt("请输入 Bitwarden 主密码")
        .interact_text()?;

    if password.is_empty() {
        Ok(None)
    } else {
        Ok(Some(password))
    }
}

fn run_generate(
    config: &mut Config,
    prefix: Option<String>,
    service: Vec<String>,
    select: bool,
    project: Option<String>,
    config_file: Option<String>,
    output: Option<String>,
    format: String,
) -> Result<()> {
    let effective_prefix = prefix
        .or_else(|| {
            project
                .as_ref()
                .and_then(|p| config.get_project_by_name(p))
                .map(|p| p.prefix.clone())
        })
        .or_else(|| config.get_current_project().map(|p| p.prefix.clone()));

    let services = if select {
        let available = if let Some(ref cf) = config_file {
            config::load_services_from_file(cf)?
        } else if let Some(project) = config.get_current_project() {
            project.services.clone()
        } else {
            return Err(anyhow!("请先配置项目或使用 --config 指定服务列表"));
        };
        select_services(&available)?
    } else if !service.is_empty() {
        service
    } else if let Some(ref cf) = config_file {
        config::load_services_from_file(cf)?
    } else if let Some(project) = config.get_current_project() {
        project.services.clone()
    } else {
        return Err(anyhow!("请先选择项目或使用 --service 指定服务"));
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

    // 如果没有当前项目，尝试从当前目录的 .bwenv 文件加载
    if config.current_project.is_none() {
        if let Ok(Some(project)) = config::Config::load_project_from_dir() {
            // 如果项目不存在于配置中，添加它
            if !config.projects.iter().any(|p| p.name == project.name) {
                config.projects.push(project.clone());
                config.save()?;
            }
            config.set_current_project(&project.name)?;
            println!("自动检测到项目: {} (来自 .bwenv)", project.name);
        }
    }

    // 处理默认命令：bwenv 直接运行 generate
    let command = cli.command.unwrap_or(Commands::Gen {
        prefix: cli.prefix,
        service: cli.service,
        select: cli.select,
        project: cli.project,
        config: cli.config,
        output: cli.output,
        format: cli.format,
    });

    match command {
        Commands::List { prefix, service, project } => {
            let effective_prefix = prefix
                .or_else(|| {
                    project
                        .as_ref()
                        .and_then(|p| config.get_project_by_name(p))
                        .map(|p| p.prefix.clone())
                })
                .or_else(|| config.get_current_project().map(|p| p.prefix.clone()));

            let master_password = get_master_password()?;
            list::list_items(master_password.as_deref(), effective_prefix.as_deref(), service.as_deref())?;
        }

        Commands::Gen {
            prefix,
            service,
            select,
            project,
            config: config_file,
            output,
            format,
        } => {
            run_generate(&mut config, prefix, service, select, project, config_file, output, format)?;
        }

        Commands::Project { command } => {
            match command {
                Some(ProjectCommands::List) => {
                    config_cmd::list_projects(&config)?;
                }
                Some(ProjectCommands::Add { name, prefix, services }) => {
                    let services: Vec<String> = services
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
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
                Some(ProjectCommands::Use { name }) => {
                    config.set_current_project(&name)?;
                    println!("已切换到项目: {}", name);
                }
                None => {
                    config_cmd::list_projects(&config)?;
                }
            }
        }

        Commands::Switch { name, list } => {
            if list {
                config_cmd::list_projects(&config)?;
            } else {
                switch::switch_command(name, &mut config)?;
            }
        }

        Commands::Current => {
            if let Some(project) = config.get_current_project() {
                println!("当前项目: {}", project.name);
                println!("前缀: {}", project.prefix);
                println!("服务: {:?}", project.services);
            } else {
                println!("未选择项目。请运行 'bwenv project use <name>' 选择项目。");
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
            // 设置当前项目并生成环境变量
            config.set_current_project(&name)?;
            println!("已切换到项目: {}", name);

            let project = config.get_current_project().ok_or_else(||
                anyhow!("项目 '{}' 不存在", name))?;

            let master_password = get_master_password()?;
            let master_password_opt = master_password.as_deref();

            generate::generate_env(
                master_password_opt,
                Some(&project.prefix),
                project.services.clone(),
                &format,
                output.as_deref(),
            )?;
        }
    }

    Ok(())
}
