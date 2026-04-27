use async_trait::async_trait;
use emdash_core::StorageProvider;

pub struct LocalStorage {
    // Add bespoke local storage config in future
}

impl LocalStorage {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl StorageProvider for LocalStorage {
    async fn get_file(&self, _path: &str) -> Result<Vec<u8>, String> {
        Ok(vec![])
    }
    async fn put_file(&self, _path: &str, _data: &[u8]) -> Result<(), String> {
        Ok(())
    }
    async fn delete_file(&self, _path: &str) -> Result<(), String> {
        Ok(())
    }
}
