use anyhow::Result;
use clap::{Parser, Subcommand};

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

        /// 指定服务名
        #[arg(short, long)]
        service: Option<String>,

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

        /// 指定服务名
        #[arg(short, long)]
        service: Option<String>,

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
                .or_else(|| config.get_current_project().map(|p| p.prefix.clone()))
                .or_else(|| config.default_prefix.clone());

            list::list_items(&mut config, effective_prefix.as_deref(), service.as_deref())?;
        }

        Commands::Generate {
            prefix,
            service,
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
                .or_else(|| config.get_current_project().map(|p| p.prefix.clone()))
                .or_else(|| config.default_prefix.clone());

            generate::generate_env(
                &mut config,
                effective_prefix.as_deref(),
                service.as_deref(),
                &format,
            )?;
        }

        Commands::Export {
            prefix,
            service,
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
                .or_else(|| config.get_current_project().map(|p| p.prefix.clone()))
                .or_else(|| config.default_prefix.clone());

            // 获取服务列表：命令行 > 配置文件 > 项目配置 > 默认配置
            let services = if service.is_some() {
                service.into_iter().collect()
            } else if let Some(ref cf) = config_file {
                config::load_services_from_file(cf)?
            } else if let Some(project) = config.get_current_project() {
                project.services.clone()
            } else {
                config.default_services.clone()
            };

            export::export_env(
                &mut config,
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
