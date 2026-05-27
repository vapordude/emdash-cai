use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Machine-readable collection definition.  Posted to `/_emdash/api/schema/collections`.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Collection {
    pub id:          Option<String>,
    pub name:        String,
    pub title:       String,
    pub description: Option<String>,
    pub is_feed:     bool,
    pub fields:      Vec<Field>,
}

/// A single field inside a collection.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Field {
    pub id:          Option<String>,
    pub name:        String,
    pub title:       String,
    pub description: Option<String>,
    pub field_type:  FieldType,
    pub required:    bool,
    pub position:    Option<u32>,
}

/// Every supported field type.  Custom types carry arbitrary JSON config.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FieldType {
    String,
    Text,
    Number,
    Boolean,
    Date,
    Datetime,
    Slug,
    Image,
    File,
    PortableText,
    Object   { fields: Vec<Field> },
    Array    { of: Box<FieldType> },
    Reference { to: String },
    Custom   { config: Value },
}

impl FieldType {
    /// Map to a SQLite column type string for DDL generation.
    pub fn to_sql_type(&self) -> &'static str {
        match self {
            FieldType::String | FieldType::Text | FieldType::Slug => "TEXT",
            FieldType::Number => "REAL",
            FieldType::Boolean => "INTEGER",
            FieldType::Date | FieldType::Datetime => "TEXT",
            FieldType::Image | FieldType::File => "TEXT",
            FieldType::PortableText => "TEXT",
            FieldType::Object { .. } | FieldType::Array { .. } => "TEXT",
            FieldType::Reference { .. } => "TEXT",
            FieldType::Custom { .. } => "TEXT",
        }
    }
}
