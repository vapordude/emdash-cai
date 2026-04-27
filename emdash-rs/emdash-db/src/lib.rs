use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait Database {
    /// Initialize the database, running migrations if needed
    async fn init(&self) -> Result<(), String>;

    /// Executes a dynamic query and returns a list of JSON objects
    async fn query(&self, sql: &str) -> Result<Vec<Value>, String>;

    /// Executes a command (e.g. INSERT, UPDATE) and returns the number of affected rows
    async fn execute(&self, sql: &str) -> Result<u64, String>;

    /// Gets a single record by ID from a dynamic table
    async fn get_by_id(&self, table: &str, id: &str) -> Result<Option<Value>, String>;

    /// Creates a new table for a given collection schema
    /// This should map standard types (String, Date, PortableText) to DB-specific columns
    async fn create_table(&self, table: &str, columns: &[ColumnDef]) -> Result<(), String>;
}

#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub col_type: ColumnType,
    pub required: bool,
    pub unique: bool,
}

#[derive(Debug, Clone)]
pub enum ColumnType {
    Text,
    Integer,
    Float,
    Boolean,
    Json,
    Timestamp,
}
