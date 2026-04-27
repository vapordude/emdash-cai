use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub fields: Vec<Field>,
}

impl Collection {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Collection name cannot be empty".to_string());
        }

        let mut field_names = std::collections::HashSet::new();
        for field in &self.fields {
            if field.name.trim().is_empty() {
                return Err("Field name cannot be empty".to_string());
            }
            if !field_names.insert(&field.name) {
                return Err(format!("Duplicate field name '{}' in collection '{}'", field.name, self.name));
            }
        }
        Ok(())
    }
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
