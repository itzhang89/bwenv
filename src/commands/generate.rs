use anyhow::{anyhow, Result};
use crate::bitwarden::client::BitwardenClient;
use crate::config::Config;
use crate::parser::env_gen::{to_env_format, to_json_format, to_shell_format, item_to_env_vars};

/// 生成环境变量（输出到 stdout）
pub fn generate_env(
    config: &mut Config,
    prefix: Option<&str>,
    service: Option<&str>,
    format: &str,
) -> Result<()> {
    let api_key = config
        .get_api_key()
        .ok_or_else(|| anyhow!("请先配置 API Key: bwenv config set api_key <key>"))?;
    let api_secret = config
        .get_api_secret()
        .ok_or_else(|| anyhow!("请先配置 API Secret: bwenv config set api_secret <secret>"))?;
    let master_password = config.get_master_password();

    let mut client = BitwardenClient::new();
    let items = client.list_items_by_folder_and_service(
        api_key,
        api_secret,
        master_password,
        prefix,
        service,
    )?;

    if items.is_empty() {
        println!("未找到匹配的 items");
        return Ok(());
    }

    // 收集所有环境变量
    let mut all_vars = Vec::new();
    for item in &items {
        let vars = item_to_env_vars(item);
        all_vars.extend(vars);
    }

    // 按格式输出
    let output = match format {
        "env" => to_env_format(&all_vars),
        "json" => to_json_format(&all_vars),
        _ => to_shell_format(&all_vars),
    };

    println!("{}", output);

    Ok(())
}
