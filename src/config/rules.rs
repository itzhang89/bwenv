use serde::{Deserialize, Serialize};

/// Field type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    User,
    Password,
    Host,
    Port,
    Database,
    ApiKey,
    Url,
    Unknown,
}

impl FieldType {
    pub fn from_field_name(name: &str) -> Self {
        let lower = name.to_lowercase();

        if lower.contains("user") || lower == "username" {
            FieldType::User
        } else if lower.contains("password") || lower == "pass" {
            FieldType::Password
        } else if lower.contains("host") || lower == "hostname" {
            FieldType::Host
        } else if lower.contains("port") {
            FieldType::Port
        } else if lower.contains("database") || lower == "dbname" || lower == "db" {
            FieldType::Database
        } else if lower.contains("key")
            || lower.contains("token")
            || lower.contains("secret")
            || lower == "api_key"
            || lower == "apikey"
        {
            FieldType::ApiKey
        } else if lower == "url" || lower.contains("url") || lower == "uri" {
            FieldType::Url
        } else {
            FieldType::Unknown
        }
    }

    pub fn suffix(&self) -> &'static str {
        match self {
            FieldType::User => "USER",
            FieldType::Password => "PASSWORD",
            FieldType::Host => "HOST",
            FieldType::Port => "PORT",
            FieldType::Database => "DATABASE",
            FieldType::ApiKey => "API_KEY",
            FieldType::Url => "URL",
            FieldType::Unknown => "",
        }
    }
}

/// Generate environment variable name from field name
pub fn generate_env_key(service_name: &str, field_name: &str) -> String {
    let field_type = FieldType::from_field_name(field_name);
    let suffix = field_type.suffix();

    // Convert service name to uppercase with underscores
    let service_key = service_name
        .to_uppercase()
        .replace('-', "_")
        .replace(' ', "_");

    if suffix.is_empty() {
        // If type cannot be identified, use original field name
        let field_key = field_name
            .to_uppercase()
            .replace('-', "_")
            .replace(' ', "_");
        format!("{}_{}", service_key, field_key)
    } else {
        format!("{}_{}", service_key, suffix)
    }
}
