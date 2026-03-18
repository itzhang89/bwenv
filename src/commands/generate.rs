use anyhow::Result;
use std::fs;
use crate::bitwarden::client::BitwardenClient;
use crate::parser::env_gen::{to_env_format, to_json_format, to_shell_format, item_to_env_vars};

/// 生成环境变量
/// services 为 None 或空时，查询全部（只按 prefix 过滤）
pub fn generate_env(
    master_password: Option<&str>,
    prefix: Option<&str>,
    services: Option<Vec<String>>,
    format: &str,
    output_path: Option<&str>,
) -> Result<()> {
    let mut client = BitwardenClient::new();

    // 收集所有服务的环境变量
    let mut all_vars = Vec::new();

    // 如果指定了 services，则按服务名筛选；否则查询全部
    if let Some(svc_list) = services {
        if !svc_list.is_empty() {
            for service in &svc_list {
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
        } else {
            // services 为空 Vec，等同于查询全部
            let items = client.list_items_by_folder_and_service(
                master_password,
                prefix,
                None,
            )?;

            for item in &items {
                let vars = item_to_env_vars(item);
                all_vars.extend(vars);
            }
        }
    } else {
        // services 为 None，查询全部
        let items = client.list_items_by_folder_and_service(
            master_password,
            prefix,
            None,
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
    let output_content = match format {
        "env" => to_env_format(&all_vars),
        "json" => to_json_format(&all_vars),
        _ => to_shell_format(&all_vars),
    };

    if let Some(path) = output_path {
        fs::write(path, &output_content)?;
        println!("已导出到: {}", path);
    } else {
        println!("{}", output_content);
    }

    Ok(())
}
