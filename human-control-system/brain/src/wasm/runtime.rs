//! Wasmtime Runtime for WASM Plugin Execution

use anyhow::{Result, Context};
use wasmtime::{
    Engine, Module, Store, Linker, Instance, Memory, Func, Val, ValType,
    Config as WasmtimeConfig, Strategy, StoreLimits, StoreLimitsBuilder,
};
use wasmtime::component::{Component, Linker as ComponentLinker, ResourceTable};
use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use tracing::{debug, info, warn, error};
use std::time::{Duration, Instant};

/// Configuration for WASM runtime
#[derive(Debug, Clone)]
pub struct WasmConfig {
    pub max_memory_mb: u32,
    pub max_fuel_per_tick: u64,
    pub fuel_per_instruction: u64,
    pub enable_component_model: bool,
    pub allowed_host_functions: Vec<String>,
    pub epoch_deadline_async_yield_and_update: bool,
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 64,
            max_fuel_per_tick: 1_000_000,
            fuel_per_instruction: 1,
            enable_component_model: true,
            allowed_host_functions: vec![
                "blackboard_get".to_string(),
                "blackboard_set".to_string(),
                "blackboard_has".to_string(),
                "log_info".to_string(),
                "log_warn".to_string(),
                "log_error".to_string(),
                "get_time_ms".to_string(),
                "random_float".to_string(),
                "random_int".to_string(),
            ],
            epoch_deadline_async_yield_and_update: true,
        }
    }
}

/// Wasmtime runtime instance
pub struct WasmtimeRuntime {
    engine: Engine,
    config: WasmConfig,
    modules: Arc<RwLock<HashMap<String, WasmModule>>>,
    component_linker: Option<ComponentLinker<WasiState>>,
}

impl WasmtimeRuntime {
    pub fn new(config: WasmConfig) -> Result<Self> {
        let mut wasmtime_config = WasmtimeConfig::new();
        wasmtime_config.consume_fuel(true);
        wasmtime_config.async_support(true);
        wasmtime_config.epoch_interruption(true);
        wasmtime_config.max_wasm_stack(512 * 1024); // 512KB stack
        
        // Set memory limits
        let max_memory_bytes = config.max_memory_mb as u64 * 1024 * 1024;
        // Memory growth is limited per store below.
        
        if config.enable_component_model {
            wasmtime_config.wasm_component_model(true);
        }
        
        let engine = Engine::new(&wasmtime_config)?;
        
        // Start epoch timer for fuel consumption
        let engine_clone = engine.clone();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(10));
                engine_clone.increment_epoch();
            }
        });
        
        Ok(Self {
            engine,
            config,
            modules: Arc::new(RwLock::new(HashMap::new())),
            component_linker: None,
        })
    }
    
    pub fn engine(&self) -> &Engine {
        &self.engine
    }
    
    pub fn config(&self) -> &WasmConfig {
        &self.config
    }
    
    /// Load a WASM module from file
    pub async fn load_module(&self, name: &str, path: &Path) -> Result<()> {
        let bytes = tokio::fs::read(path).await
            .context(format!("Failed to read WASM file: {}", path.display()))?;
        
        let module = Module::new(&self.engine, &bytes)
            .context("Failed to compile WASM module")?;
        
        let wasm_module = WasmModule {
            name: name.to_string(),
            module,
            path: path.to_path_buf(),
            loaded_at: Instant::now(),
        };
        
        self.modules.write().insert(name.to_string(), wasm_module);
        info!(name, path = %path.display(), "Loaded WASM module");
        
        Ok(())
    }
    
    /// Load a WASM component from file
    pub async fn load_component(&self, name: &str, path: &Path) -> Result<()> {
        if !self.config.enable_component_model {
            return Err(anyhow::anyhow!("Component model not enabled"));
        }
        
        let bytes = tokio::fs::read(path).await
            .context(format!("Failed to read WASM component: {}", path.display()))?;
        
        let component = Component::new(&self.engine, &bytes)
            .context("Failed to compile WASM component")?;
        
        let wasm_module = WasmModule {
            name: name.to_string(),
            module: Module::new(&self.engine, &bytes)?, // Fallback
            path: path.to_path_buf(),
            loaded_at: Instant::now(),
        };
        
        self.modules.write().insert(name.to_string(), wasm_module);
        info!(name, path = %path.display(), "Loaded WASM component");
        
        Ok(())
    }
    
    /// Create a new store with fuel limit
    pub fn create_store(&self) -> Store<WasiState> {
        let mut store = Store::new(&self.engine, WasiState::new());
        store.set_fuel(self.config.max_fuel_per_tick).expect("fuel enabled");
        store.data_mut().limits = StoreLimitsBuilder::new().memory_size(self.config.max_memory_mb as usize * 1024 * 1024).build();
        store.limiter(|state| &mut state.limits);
        store.set_epoch_deadline(100);
        store
    }
    
    /// Instantiate a module with host functions
    pub async fn instantiate(&self, store: &mut Store<WasiState>, module_name: &str) -> Result<Instance> {
        let modules = self.modules.read();
        let wasm_module = modules.get(module_name)
            .ok_or_else(|| anyhow::anyhow!("Module not found: {}", module_name))?;
        
        let module = wasm_module.module.clone();
        drop(modules);
        let mut linker = Linker::new(&self.engine);
        
        // Add host functions
        crate::wasm::host_functions::HostFunctions::new(store.data().blackboard.clone()).add_to_linker(&mut linker)?;
        
        let instance = linker.instantiate_async(&mut *store, &module).await?;
        Ok(instance)
    }
    
    /// Add host functions to linker
    fn add_host_functions(&self, linker: &mut Linker<WasiState>) -> Result<()> {
        // Blackboard get
        linker.func_wrap("host", "blackboard_get", |mut caller: wasmtime::Caller<'_, WasiState>, key_ptr: i32, key_len: i32| -> i32 {
            // Implementation would read from caller memory
            0
        })?;
        
        // Blackboard set
        linker.func_wrap("host", "blackboard_set", |mut caller: wasmtime::Caller<'_, WasiState>, key_ptr: i32, key_len: i32, val_ptr: i32, val_len: i32| -> i32 {
            0
        })?;
        
        // Log functions
        linker.func_wrap("host", "log_info", |msg_ptr: i32, msg_len: i32| {
            // Implementation
        })?;
        
        linker.func_wrap("host", "log_warn", |msg_ptr: i32, msg_len: i32| {
            // Implementation
        })?;
        
        linker.func_wrap("host", "log_error", |msg_ptr: i32, msg_len: i32| {
            // Implementation
        })?;
        
        // Time
        linker.func_wrap("host", "get_time_ms", || -> i64 {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64
        })?;
        
        // Random
        linker.func_wrap("host", "random_float", || -> f64 {
            use rand::Rng;
            rand::thread_rng().gen()
        })?;
        
        linker.func_wrap("host", "random_int", |min: i32, max: i32| -> i32 {
            use rand::Rng;
            rand::thread_rng().gen_range(min..max)
        })?;
        
        Ok(())
    }
    
    /// Get loaded module names
    pub fn list_modules(&self) -> Vec<String> {
        self.modules.read().keys().cloned().collect()
    }
    
    /// Unload a module
    pub fn unload_module(&self, name: &str) -> bool {
        self.modules.write().remove(name).is_some()
    }
    
    /// Check if module exists
    pub fn has_module(&self, name: &str) -> bool {
        self.modules.read().contains_key(name)
    }
}

/// Loaded WASM module
#[derive(Debug)]
pub struct WasmModule {
    pub name: String,
    pub module: Module,
    pub path: std::path::PathBuf,
    pub loaded_at: Instant,
}

/// WASI state for host functions
pub struct WasiState {
    pub limits: StoreLimits,
    pub blackboard: Arc<RwLock<crate::bt::Blackboard>>,
    pub resource_table: ResourceTable,
}

impl WasiState {
    pub fn new() -> Self {
        Self {
            blackboard: Arc::new(RwLock::new(crate::bt::Blackboard::new())),
            resource_table: ResourceTable::new(),
            limits: StoreLimitsBuilder::new().build(),
        }
    }
}

impl Default for WasiState {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin instance for execution
pub struct PluginInstance {
    pub name: String,
    pub instance: Instance,
    pub store: Store<WasiState>,
    pub init_func: Option<Func>,
    pub tick_func: Option<Func>,
    pub shutdown_func: Option<Func>,
    pub action_funcs: HashMap<String, Func>,
    pub condition_funcs: HashMap<String, Func>,
}

impl PluginInstance {
    pub fn new(name: String, instance: Instance, mut store: Store<WasiState>) -> Result<Self> {
        let init_func = instance.get_func(&mut store, "init");
        let tick_func = instance.get_func(&mut store, "tick");
        let shutdown_func = instance.get_func(&mut store, "shutdown");
        
        // Get action and condition exports
        let mut action_funcs = HashMap::new();
        let mut condition_funcs = HashMap::new();
        
        for export in instance.exports(&mut store) {
            let name = export.name().to_owned();
            if name.starts_with("action_") {
                if let Some(func) = export.into_func() {
                    action_funcs.insert(name[7..].to_string(), func);
                }
            } else if name.starts_with("condition_") {
                if let Some(func) = export.into_func() {
                    condition_funcs.insert(name[10..].to_string(), func);
                }
            }
        }
        
        Ok(Self {
            name,
            instance,
            store,
            init_func,
            tick_func,
            shutdown_func,
            action_funcs,
            condition_funcs,
        })
    }
    
    pub async fn call_init(&mut self, config: &serde_json::Value) -> Result<()> {
        if let Some(func) = self.init_func {
            let config_json = serde_json::to_string(config)?;
            // Would need to write to WASM memory and call
            // Simplified for now
            func.call_async(&mut self.store, &[], &mut []).await?;
        }
        Ok(())
    }
    
    pub async fn call_tick(&mut self, blackboard: &crate::bt::Blackboard) -> Result<NodeStatus> {
        if let Some(func) = self.tick_func {
            // Write blackboard to WASM memory, call tick, read result
            // Simplified for now
            let mut results = [Val::I32(0)];
            func.call_async(&mut self.store, &[], &mut results).await?;
            let status_val = results[0].unwrap_i32();
            Ok(match status_val {
                0 => NodeStatus::Pending,
                1 => NodeStatus::Running,
                2 => NodeStatus::Success,
                3 => NodeStatus::Failure,
                4 => NodeStatus::Skipped,
                _ => NodeStatus::Error,
            })
        } else {
            Ok(NodeStatus::Error)
        }
    }
    
    pub async fn call_shutdown(&mut self) -> Result<()> {
        if let Some(func) = self.shutdown_func {
            func.call_async(&mut self.store, &[], &mut []).await?;
        }
        Ok(())
    }
    
    pub async fn call_action(&mut self, action_name: &str, params: &HashMap<String, serde_json::Value>) -> Result<NodeStatus> {
        if let Some(func) = self.action_funcs.get(action_name) {
            // Write params to WASM memory, call, read result
            let mut results = [Val::I32(0)];
            func.call_async(&mut self.store, &[], &mut results).await?;
            let status_val = results[0].unwrap_i32();
            Ok(match status_val {
                0 => NodeStatus::Pending,
                1 => NodeStatus::Running,
                2 => NodeStatus::Success,
                3 => NodeStatus::Failure,
                4 => NodeStatus::Skipped,
                _ => NodeStatus::Error,
            })
        } else {
            Err(anyhow::anyhow!("Action not found: {}", action_name))
        }
    }
    
    pub async fn call_condition(&mut self, condition_name: &str, params: &HashMap<String, serde_json::Value>) -> Result<bool> {
        if let Some(func) = self.condition_funcs.get(condition_name) {
            let mut results = [Val::I32(0)];
            func.call_async(&mut self.store, &[], &mut results).await?;
            Ok(results[0].unwrap_i32() != 0)
        } else {
            Err(anyhow::anyhow!("Condition not found: {}", condition_name))
        }
    }
}

use crate::bt::NodeStatus;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wasm_config_default() {
        let config = WasmConfig::default();
        assert_eq!(config.max_memory_mb, 64);
        assert_eq!(config.max_fuel_per_tick, 1_000_000);
        assert!(config.enable_component_model);
    }
    
    #[tokio::test]
    async fn test_runtime_creation() {
        let config = WasmConfig::default();
        let runtime = WasmtimeRuntime::new(config);
        assert!(runtime.is_ok());
    }
}
impl From<crate::config::WasmConfig> for WasmConfig {
    fn from(config: crate::config::WasmConfig) -> Self {
        Self { max_memory_mb: config.max_memory_mb, max_fuel_per_tick: config.max_fuel_per_tick,
            fuel_per_instruction: config.fuel_per_instruction, allowed_host_functions: config.allowed_host_functions,
            ..Self::default() }
    }
}
