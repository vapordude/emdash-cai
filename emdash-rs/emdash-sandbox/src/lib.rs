use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait PluginRunner {
    /// Loads a plugin (e.g. Wasm module) into the sandbox
    async fn load_plugin(&self, plugin_id: &str, wasm_bytes: &[u8]) -> Result<(), String>;

    /// Executes a hook/function within the plugin sandbox securely
    async fn execute_hook(
        &self,
        plugin_id: &str,
        hook_name: &str,
        payload: Value,
    ) -> Result<Value, String>;

    /// Removes a plugin from the sandbox
    async fn unload_plugin(&self, plugin_id: &str) -> Result<(), String>;
}

#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub max_memory_mb: u32,
    pub allow_network: bool,
    pub allow_db_read: bool,
    pub allow_db_write: bool,
}
