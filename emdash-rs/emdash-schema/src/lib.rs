use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub field_type: FieldType,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FieldType {
    String,
    Text,
    Number,
    Boolean,
    Date,
    Datetime,
    Object { fields: Vec<Field> },
    Array { of: Box<FieldType> },
    Reference { to: String },
    PortableText,
}
