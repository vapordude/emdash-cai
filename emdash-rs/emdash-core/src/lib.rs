pub mod portable_text;

use async_trait::async_trait;

#[async_trait]
pub trait StorageProvider {
    async fn get_file(&self, path: &str) -> Result<Vec<u8>, String>;
    async fn put_file(&self, path: &str, data: &[u8]) -> Result<(), String>;
    async fn delete_file(&self, path: &str) -> Result<(), String>;
}

#[async_trait]
pub trait DatabaseProvider {
    async fn query(&self, query: &str) -> Result<Vec<serde_json::Value>, String>;
    async fn execute(&self, query: &str) -> Result<u64, String>;
}

#[async_trait]
pub trait LlmProvider {
    async fn generate_text(&self, prompt: &str) -> Result<String, String>;
}
