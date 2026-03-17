use anyhow::Result;
use crate::bitwarden::client::BitwardenClient;

/// 列出 Bitwarden vault 中的 items
pub fn list_items(master_password: Option<&str>, prefix: Option<&str>, service: Option<&str>) -> Result<()> {
    let mut client = BitwardenClient::new();
    let items = client.list_items_by_folder_and_service(
        master_password,
        prefix,
        service,
    )?;

    if items.is_empty() {
        println!("未找到匹配的 items");
        return Ok(());
    }

    println!("找到 {} 个 items:\n", items.len());
    for item in &items {
        println!("  - {}", item.get_name().unwrap_or("(无名称)"));
        // 处理 login
        if let Some(login_obj) = item.login.as_object() {
            if let Some(username) = login_obj.get("username").and_then(|v| v.as_str()) {
                println!("    用户名: {}", username);
            }
        }
        // 处理自定义字段
        if let Some(fields_arr) = item.fields.as_array() {
            for field_val in fields_arr {
                if let Some(field_obj) = field_val.as_object() {
                    if let Some(name) = field_obj.get("name").and_then(|v| v.as_str()) {
                        println!("    {}: ****", name);
                    }
                }
            }
        }
        println!();
    }

    Ok(())
}
