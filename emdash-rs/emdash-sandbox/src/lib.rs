use async_trait::async_trait;
use emdash_core::PluginRunner;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub max_memory_mb: u32,
    pub allow_network: bool,
    pub allow_db_read: bool,
    pub allow_db_write: bool,
}

pub struct WasmPluginRunner {
    config: SandboxConfig,
}

impl WasmPluginRunner {
    pub fn new(config: SandboxConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl PluginRunner for WasmPluginRunner {
    async fn load_plugin(&self, _plugin_id: &str, _wasm_bytes: &[u8]) -> Result<(), String> {
        Ok(())
    }

    async fn execute_hook(
        &self,
        _plugin_id: &str,
        _hook_name: &str,
        _payload: Value,
    ) -> Result<Value, String> {
        Ok(Value::Null)
    }

    async fn unload_plugin(&self, _plugin_id: &str) -> Result<(), String> {
        Ok(())
    }
}
