use async_trait::async_trait;
use emdash_core::ApiError;
use serde_json::Value;

// ── Traits ────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait PluginRunner: Send + Sync {
    /// Compile and cache a Wasm module.
    async fn load_plugin(&self, plugin_id: &str, wasm_bytes: &[u8]) -> Result<(), ApiError>;

    /// Invoke a named hook inside the plugin's sandbox and return the result.
    async fn execute_hook(
        &self,
        plugin_id: &str,
        hook_name: &str,
        payload: Value,
    ) -> Result<Value, ApiError>;

    /// Remove a plugin from the cache.
    async fn unload_plugin(&self, plugin_id: &str) -> Result<(), ApiError>;

    /// List loaded plugin IDs.
    async fn loaded_plugins(&self) -> Vec<String>;
}

/// Capability grants for a plugin sandbox.
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub max_memory_mb: u32,
    pub allow_network: bool,
    pub allow_db_read: bool,
    pub allow_db_write: bool,
    pub allow_storage: bool,
    pub allow_llm: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 16,
            allow_network: false,
            allow_db_read: true,
            allow_db_write: false,
            allow_storage: false,
            allow_llm: false,
        }
    }
}

// ── No-op runner (always compiled) ───────────────────────────────────────────

/// Placeholder runner used when the `wasmtime-sandbox` feature is disabled.
/// Returns an error for any operation so the caller knows sandboxing is off.
pub struct NoopPluginRunner;

#[async_trait]
impl PluginRunner for NoopPluginRunner {
    async fn load_plugin(&self, _id: &str, _bytes: &[u8]) -> Result<(), ApiError> {
        Err(ApiError::Internal(
            "plugin sandbox not enabled (compile with --features wasmtime-sandbox)".into(),
        ))
    }

    async fn execute_hook(
        &self,
        _id: &str,
        _hook: &str,
        _payload: Value,
    ) -> Result<Value, ApiError> {
        Err(ApiError::Internal(
            "plugin sandbox not enabled (compile with --features wasmtime-sandbox)".into(),
        ))
    }

    async fn unload_plugin(&self, _id: &str) -> Result<(), ApiError> {
        Ok(())
    }

    async fn loaded_plugins(&self) -> Vec<String> {
        vec![]
    }
}

// ── Wasmtime runner (feature-gated) ──────────────────────────────────────────

#[cfg(feature = "wasmtime-sandbox")]
pub mod wasm {
    use super::*;
    use dashmap::DashMap;
    use wasmtime::{Engine, Linker, Module, Store};

    /// Wasmtime-based sandbox.  Modules are compiled once and cached.
    pub struct WasmPluginRunner {
        engine: Engine,
        modules: DashMap<String, Module>,
        config: SandboxConfig,
    }

    impl WasmPluginRunner {
        pub fn new(config: SandboxConfig) -> Result<Self, ApiError> {
            let engine = Engine::default();
            Ok(Self {
                engine,
                modules: DashMap::new(),
                config,
            })
        }
    }

    #[async_trait]
    impl PluginRunner for WasmPluginRunner {
        async fn load_plugin(&self, plugin_id: &str, wasm_bytes: &[u8]) -> Result<(), ApiError> {
            let module = Module::new(&self.engine, wasm_bytes)
                .map_err(|e| ApiError::Internal(format!("wasm compile {plugin_id}: {e}")))?;
            self.modules.insert(plugin_id.to_string(), module);
            tracing::info!(plugin_id, "plugin loaded");
            Ok(())
        }

        async fn execute_hook(
            &self,
            plugin_id: &str,
            hook_name: &str,
            payload: Value,
        ) -> Result<Value, ApiError> {
            let module = self
                .modules
                .get(plugin_id)
                .ok_or_else(|| ApiError::NotFound(format!("plugin '{plugin_id}' not loaded")))?;

            let mut linker: Linker<()> = Linker::new(&self.engine);

            // Expose host functions gated by capability flags.
            // Each host fn is only linked if the capability is granted.
            if self.config.allow_db_read {
                linker
                    .func_wrap(
                        "env",
                        "db_query",
                        |_caller: wasmtime::Caller<'_, ()>| -> i32 { 0 },
                    )
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
            }
            if self.config.allow_db_write {
                linker
                    .func_wrap(
                        "env",
                        "db_execute",
                        |_caller: wasmtime::Caller<'_, ()>| -> i32 { 0 },
                    )
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
            }

            let mut store = Store::new(&self.engine, ());
            let instance = linker
                .instantiate(&mut store, &module)
                .map_err(|e| ApiError::Internal(format!("wasm instantiate {plugin_id}: {e}")))?;

            // Plugins export a function named after the hook.
            let func = instance
                .get_typed_func::<(), i32>(&mut store, hook_name)
                .map_err(|_| {
                    ApiError::NotFound(format!("hook '{hook_name}' not exported by '{plugin_id}'"))
                })?;

            let _result = func.call(&mut store, ()).map_err(|e| {
                ApiError::Internal(format!("wasm trap {plugin_id}/{hook_name}: {e}"))
            })?;

            tracing::info!(plugin_id, hook_name, "hook executed");
            // Return the payload unchanged for now; full data exchange via Wasm
            // memory will be wired in a follow-up once the ABI is settled.
            Ok(payload)
        }

        async fn unload_plugin(&self, plugin_id: &str) -> Result<(), ApiError> {
            self.modules.remove(plugin_id);
            tracing::info!(plugin_id, "plugin unloaded");
            Ok(())
        }

        async fn loaded_plugins(&self) -> Vec<String> {
            self.modules.iter().map(|e| e.key().clone()).collect()
        }
    }
}
