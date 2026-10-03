//! Plugin Trait and Executor

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, info, warn, error};
use crate::bt::{Blackboard, NodeStatus};

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub exports: PluginExports,
    pub dependencies: Vec<String>,
}

/// Plugin exports (actions and conditions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginExports {
    pub actions: Vec<ActionExport>,
    pub conditions: Vec<ConditionExport>,
}

/// Action export definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionExport {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value, // JSON Schema
}

/// Condition export definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionExport {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value, // JSON Schema
}

/// Plugin interface that all WASM plugins must implement
#[async_trait]
pub trait PluginInterface: Send + Sync {
    /// Initialize the plugin with configuration
    async fn init(&mut self, config: &serde_json::Value) -> Result<()>;
    
    /// Execute a single tick
    async fn tick(&mut self, blackboard: &Blackboard) -> Result<NodeStatus>;
    
    /// Shutdown the plugin
    async fn shutdown(&mut self) -> Result<()>;
    
    /// Execute an action by name
    async fn execute_action(&mut self, name: &str, params: &HashMap<String, serde_json::Value>, blackboard: &Blackboard) -> Result<NodeStatus>;
    
    /// Evaluate a condition by name
    async fn evaluate_condition(&mut self, name: &str, params: &HashMap<String, serde_json::Value>, blackboard: &Blackboard) -> Result<bool>;
    
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;
}

/// Plugin executor that manages plugin lifecycle
pub struct PluginExecutor {
    runtime: Arc<crate::wasm::runtime::WasmtimeRuntime>,
    plugins: Arc<RwLock<HashMap<String, PluginInstance>>>,
    metadata: Arc<RwLock<HashMap<String, PluginMetadata>>>,
}

impl PluginExecutor {
    pub fn new(runtime: Arc<crate::wasm::runtime::WasmtimeRuntime>) -> Self {
        Self {
            runtime,
            plugins: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Load a plugin from a WASM file
    pub async fn load_plugin(&self, name: &str, path: &Path) -> Result<()> {
        // Load the module
        self.runtime.load_module(name, path).await?;
        
        // Create store and instantiate
        let mut store = self.runtime.create_store();
        let instance = self.runtime.instantiate(&mut store, name).await?;
        
        let plugin_instance = PluginInstance::new(crate::wasm::runtime::PluginInstance::new(name.to_string(), instance, store)?);
        
        // Call init
        plugin_instance.call_init(&serde_json::Value::Null).await?;
        
        // Store plugin
        self.plugins.write().insert(name.to_string(), plugin_instance);
        info!(name, "Plugin loaded successfully");
        
        Ok(())
    }
    
    /// Load a plugin component
    pub async fn load_component(&self, name: &str, path: &Path) -> Result<()> {
        self.runtime.load_component(name, path).await?;
        info!(name, "Plugin component loaded");
        Ok(())
    }
    
    /// Unload a plugin
    pub async fn unload_plugin(&self, name: &str) -> Result<()> {
        let removed = self.plugins.write().remove(name);
        if let Some(plugin) = removed {
            plugin.call_shutdown().await?;
            self.runtime.unload_module(name);
            info!(name, "Plugin unloaded");
        }
        self.metadata.write().remove(name);
        Ok(())
    }
    
    /// Reload a plugin (hot reload)
    pub async fn reload_plugin(&self, name: &str, path: &Path) -> Result<()> {
        self.unload_plugin(name).await?;
        self.load_plugin(name, path).await
    }
    
    /// Execute a plugin action
    pub async fn execute_action(
        &self,
        plugin_name: &str,
        action_name: &str,
        blackboard: &Blackboard,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<NodeStatus> {
        let plugin = self.plugins.read().get(plugin_name).cloned()
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", plugin_name))?;
        
        plugin.call_action(action_name, params).await
    }
    
    /// Evaluate a plugin condition
    pub async fn evaluate_condition(
        &self,
        plugin_name: &str,
        condition_name: &str,
        blackboard: &Blackboard,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<bool> {
        let plugin = self.plugins.read().get(plugin_name).cloned()
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", plugin_name))?;
        
        plugin.call_condition(condition_name, params).await
    }
    
    /// Tick all plugins
    pub async fn tick_all(&self, blackboard: &Blackboard) -> HashMap<String, NodeStatus> {
        let mut results = HashMap::new();
        let plugins = self.plugins.read().clone();
        
        for (name, plugin) in plugins.iter() {
            match plugin.call_tick(blackboard).await {
                Ok(status) => {
                    results.insert(name.clone(), status);
                }
                Err(e) => {
                    error!(plugin = %name, error = %e, "Plugin tick failed");
                    results.insert(name.clone(), NodeStatus::Error);
                }
            }
        }
        
        results
    }
    
    /// Get plugin metadata
    pub fn get_metadata(&self, name: &str) -> Option<PluginMetadata> {
        self.metadata.read().get(name).cloned()
    }
    
    /// List all loaded plugins
    pub fn list_plugins(&self) -> Vec<String> {
        self.plugins.read().keys().cloned().collect()
    }
    
    /// Check if plugin is loaded
    pub fn has_plugin(&self, name: &str) -> bool {
        self.plugins.read().contains_key(name)
    }
}

/// Plugin instance wrapper for the executor
#[derive(Clone)]
pub struct PluginInstance {
    inner: Arc<tokio::sync::Mutex<crate::wasm::runtime::PluginInstance>>,
}

impl PluginInstance {
    pub fn new(inner: crate::wasm::runtime::PluginInstance) -> Self {
        Self {
            inner: Arc::new(tokio::sync::Mutex::new(inner)),
        }
    }
    
    pub async fn call_init(&self, config: &serde_json::Value) -> Result<()> {
        self.inner.lock().await.call_init(config).await
    }
    
    pub async fn call_tick(&self, blackboard: &Blackboard) -> Result<NodeStatus> {
        self.inner.lock().await.call_tick(blackboard).await
    }
    
    pub async fn call_shutdown(&self) -> Result<()> {
        self.inner.lock().await.call_shutdown().await
    }
    
    pub async fn call_action(&self, action_name: &str, params: &HashMap<String, serde_json::Value>) -> Result<NodeStatus> {
        self.inner.lock().await.call_action(action_name, params).await
    }
    
    pub async fn call_condition(&self, condition_name: &str, params: &HashMap<String, serde_json::Value>) -> Result<bool> {
        self.inner.lock().await.call_condition(condition_name, params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_plugin_metadata() {
        let metadata = PluginMetadata {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test plugin".to_string(),
            author: "Test".to_string(),
            exports: PluginExports {
                actions: vec![],
                conditions: vec![],
            },
            dependencies: vec![],
        };
        
        assert_eq!(metadata.name, "test");
    }
}