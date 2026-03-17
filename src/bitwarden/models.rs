use serde::Deserialize;

pub type JsonValue = serde_json::Value;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct BitwardenItem {
    #[serde(default)]
    pub id: JsonValue,
    #[serde(default)]
    pub name: JsonValue,
    #[serde(rename = "type", default)]
    pub item_type: JsonValue,
    #[serde(default)]
    pub login: JsonValue,
    #[serde(default)]
    pub secure_note: JsonValue,
    #[serde(default)]
    pub fields: JsonValue,
    #[serde(rename = "folderId", default)]
    pub folder_id: JsonValue,
    #[serde(rename = "passwordHistory", default)]
    pub password_history: JsonValue,
    #[serde(rename = "deletedDate", default)]
    pub deleted_date: JsonValue,
    #[serde(rename = "organizationId", default)]
    pub organization_id: JsonValue,
    #[serde(default)]
    pub notes: JsonValue,
    #[serde(default)]
    pub revision_date: JsonValue,
    #[serde(rename = "creationDate", default)]
    pub creation_date: JsonValue,
    #[serde(default)]
    pub favorite: JsonValue,
    #[serde(default)]
    pub reprompt: JsonValue,
    #[serde(rename = "collectionIds", default)]
    pub collection_ids: JsonValue,
    #[serde(default)]
    pub object: JsonValue,
}

impl BitwardenItem {
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_str()
    }
}

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct Login {
    #[serde(default)]
    pub username: JsonValue,
    #[serde(default)]
    pub password: JsonValue,
    #[serde(rename = "uris", default)]
    pub uris: JsonValue,
    #[serde(rename = "totp", default)]
    pub totp: JsonValue,
    #[serde(rename = "fido2Credentials", default)]
    pub fido2_credentials: JsonValue,
    #[serde(rename = "passwordRevisionDate", default)]
    pub password_revision_date: JsonValue,
}

impl Login {
    pub fn get_username(&self) -> Option<&str> {
        self.username.as_str()
    }

    pub fn get_password(&self) -> Option<&str> {
        self.password.as_str()
    }

    pub fn get_uri(&self) -> Option<String> {
        if let Some(arr) = self.uris.as_array() {
            if let Some(first) = arr.first() {
                if let Some(obj) = first.as_object() {
                    if let Some(uri) = obj.get("uri") {
                        return uri.as_str().map(|s| s.to_string());
                    }
                }
            }
        }
        None
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Default)]
pub struct SecureNote {
    #[serde(default)]
    pub notes: JsonValue,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Default)]
pub struct Field {
    #[serde(default)]
    pub name: JsonValue,
    #[serde(default)]
    pub value: JsonValue,
    #[serde(rename = "type", default)]
    pub field_type: JsonValue,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct BitwardenFolder {
    pub id: JsonValue,
    pub name: JsonValue,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct BitwardenListResponse<T> {
    pub data: Vec<T>,
}
