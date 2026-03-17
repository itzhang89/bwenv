use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use dialoguer::Input;

mod bitwarden;
mod commands;
mod config;
mod parser;

use commands::{config_cmd, export, generate, list, switch};
use config::Config;

#[derive(Parser)]
#[command(name = "bwenv")]
#[command(about = "Bitwarden to Environment Variables Tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 列出 Bitwarden vault 中的 items
    List {
        /// 指定前缀筛选（如 dev, prod）
        #[arg(short, long)]
        prefix: Option<String>,

        /// 指定服务名筛选
        #[arg(short, long)]
        service: Option<String>,

        /// 指定项目名（覆盖当前项目）
        #[arg(long)]
        project: Option<String>,
    },

    /// 生成环境变量（输出到 stdout）
    Generate {
        /// 指定前缀筛选
        #[arg(short, long)]
        prefix: Option<String>,

        /// 指定服务名（可多次指定）
        #[arg(short, long)]
        service: Vec<String>,

        /// 交互式选择服务
        #[arg(long)]
        select: bool,

        /// 指定项目名（覆盖当前项目）
        #[arg(long)]
        project: Option<String>,

        /// 输出格式：shell, env, json
        #[arg(short, long, default_value = "shell")]
        format: String,
    },

    /// 导出环境变量到文件
    Export {
        /// 指定前缀筛选
        #[arg(short, long)]
        prefix: Option<String>,

        /// 指定服务名（可多次指定）
        #[arg(short, long)]
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
    },

    /// 切换当前项目/目录
    Switch {
        /// 项目名称（可选，不提供则交互式选择）
        name: Option<String>,

        /// 列出已配置的项目
        #[arg(long)]
        list: bool,
    },

    /// 查看当前项目
    Current,

    /// 配置管理
    #[command(subcommand)]
    Config(config_cmd::ConfigCommands),
}

/// 交互式选择多个服务
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

/// 获取 master password，必要时提示用户输入
fn get_master_password() -> Result<Option<String>> {
    // 检查环境变量
    if let Ok(password) = std::env::var("BW_MASTER_PASSWORD") {
        if !password.is_empty() {
            return Ok(Some(password));
        }
    }

    // 尝试从配置文件读取
    if let Ok(config) = Config::load() {
        if let Some(password) = config.get_master_password() {
            return Ok(Some(password.to_string()));
        }
    }

    // 未配置密码，提示用户输入
    let password: String = Input::new()
        .with_prompt("请输入 Bitwarden 主密码")
        .interact_text()?;

    if password.is_empty() {
        Ok(None)
    } else {
        Ok(Some(password))
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // 加载配置
    let mut config = Config::load()?;

    match cli.command {
        Commands::List {
            prefix,
            service,
            project,
        } => {
            // 命令行参数优先，其次项目配置，最后默认
            let effective_prefix = prefix
                .or_else(|| {
                    project
                        .as_ref()
                        .and_then(|p| config.get_project_by_name(p))
                        .map(|p| p.prefix.clone())
                })
                .or_else(|| config.get_current_project().map(|p| p.prefix.clone()));

            // 获取 master password
            let master_password = get_master_password()?;
            let master_password_opt = master_password.as_deref();

            list::list_items(master_password_opt, effective_prefix.as_deref(), service.as_deref())?;
        }

        Commands::Generate {
            prefix,
            service,
            select,
            project,
            format,
        } => {
            let effective_prefix = prefix
                .or_else(|| {
                    project
                        .as_ref()
                        .and_then(|p| config.get_project_by_name(p))
                        .map(|p| p.prefix.clone())
                })
                .or_else(|| config.get_current_project().map(|p| p.prefix.clone()));

            // 获取服务列表
            let services = if select {
                // 交互式选择
                let available = config.default_services.clone();
                select_services(&available)?
            } else if !service.is_empty() {
                service
            } else if let Some(project) = config.get_current_project() {
                project.services.clone()
            } else {
                config.default_services.clone()
            };

            // 获取 master password
            let master_password = get_master_password()?;
            let master_password_opt = master_password.as_deref();

            generate::generate_env(
                master_password_opt,
                effective_prefix.as_deref(),
                services,
                &format,
            )?;
        }

        Commands::Export {
            prefix,
            service,
            select,
            project,
            config: config_file,
            output,
            format,
        } => {
            let effective_prefix = prefix
                .or_else(|| {
                    project
                        .as_ref()
                        .and_then(|p| config.get_project_by_name(p))
                        .map(|p| p.prefix.clone())
                })
                .or_else(|| config.get_current_project().map(|p| p.prefix.clone()));

            // 获取服务列表
            let services = if select {
                // 交互式选择
                let available = if let Some(ref cf) = config_file {
                    config::load_services_from_file(cf)?
                } else if let Some(project) = config.get_current_project() {
                    project.services.clone()
                } else {
                    config.default_services.clone()
                };
                select_services(&available)?
            } else if !service.is_empty() {
                service
            } else if let Some(ref cf) = config_file {
                config::load_services_from_file(cf)?
            } else if let Some(project) = config.get_current_project() {
                project.services.clone()
            } else {
                config.default_services.clone()
            };

            // 获取 master password
            let master_password = get_master_password()?;
            let master_password_opt = master_password.as_deref();

            export::export_env(
                master_password_opt,
                effective_prefix.as_deref(),
                services,
                &format,
                output.as_deref(),
            )?;
        }

        Commands::Switch { name, list } => {
            if list {
                switch::list_projects(&config)?;
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
                println!("未选择项目。请运行 'bwenv switch' 选择项目。");
            }
        }

        Commands::Config(cmd) => {
            config_cmd::config_command(cmd, &mut config)?;
        }
    }

    Ok(())
}
