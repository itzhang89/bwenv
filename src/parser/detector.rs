use crate::config::rules::FieldType;

/// 检测字段类型
#[allow(dead_code)]
pub fn detect_field_type(name: &str) -> FieldType {
    FieldType::from_field_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_user() {
        assert_eq!(detect_field_type("username"), FieldType::User);
        assert_eq!(detect_field_type("db_user"), FieldType::User);
        assert_eq!(detect_field_type("mysql_user"), FieldType::User);
    }

    #[test]
    fn test_detect_password() {
        assert_eq!(detect_field_type("password"), FieldType::Password);
        assert_eq!(detect_field_type("db_password"), FieldType::Password);
        assert_eq!(detect_field_type("pass"), FieldType::Password);
    }

    #[test]
    fn test_detect_host() {
        assert_eq!(detect_field_type("host"), FieldType::Host);
        assert_eq!(detect_field_type("hostname"), FieldType::Host);
        assert_eq!(detect_field_type("db_host"), FieldType::Host);
    }

    #[test]
    fn test_detect_api_key() {
        assert_eq!(detect_field_type("api_key"), FieldType::ApiKey);
        assert_eq!(detect_field_type("token"), FieldType::ApiKey);
        assert_eq!(detect_field_type("secret"), FieldType::ApiKey);
    }
}
