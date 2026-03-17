use anyhow::Result;
use std::fs;
use crate::bitwarden::client::BitwardenClient;
use crate::parser::env_gen::{to_env_format, to_json_format, to_shell_format, item_to_env_vars};

/// 导出环境变量到文件
pub fn export_env(
    master_password: Option<&str>,
    prefix: Option<&str>,
    services: Vec<String>,
    format: &str,
    output_path: Option<&str>,
) -> Result<()> {
    let mut client = BitwardenClient::new();

    // 收集所有环境变量
    let mut all_vars = Vec::new();

    for service in &services {
        let items = client.list_items_by_folder_and_service(
            master_password,
            prefix,
            Some(service),
        )?;

        for item in &items {
            let vars = item_to_env_vars(item);
            all_vars.extend(vars);
        }
    }

    if all_vars.is_empty() {
        println!("未找到匹配的 items");
        return Ok(());
    }

    // 按格式输出
    let output = match format {
        "env" => to_env_format(&all_vars),
        "json" => to_json_format(&all_vars),
        _ => to_shell_format(&all_vars),
    };

    if let Some(path) = output_path {
        fs::write(path, &output)?;
        println!("已导出到: {}", path);
    } else {
        println!("{}", output);
    }

    Ok(())
}
