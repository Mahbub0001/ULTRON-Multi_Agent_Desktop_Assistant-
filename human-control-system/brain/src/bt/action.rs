//! Action nodes: executable actions that perform work

use crate::bt::{NodeId, NodeStatus, Blackboard, ExecutionContext, NodeConfig};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Trait for action node execution
#[async_trait]
pub trait ActionNode: Send + Sync {
    /// Execute the action
    async fn execute(&self, ctx: &mut ExecutionContext, params: &HashMap<String, serde_json::Value>) -> NodeStatus;
    
    /// Get the action name
    fn name(&self) -> &str;
    
    /// Get parameter schema (optional)
    fn parameter_schema(&self) -> Option<serde_json::Value> {
        None
    }
}

/// Built-in action registry
pub struct ActionRegistry {
    actions: HashMap<String, Arc<dyn ActionNode>>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            actions: HashMap::new(),
        };
        registry.register_builtins();
        registry
    }
    
    fn register_builtins(&mut self) {
        // Register built-in actions
        self.register(Box::new(WaitAction));
        self.register(Box::new(LogAction));
        self.register(Box::new(SetBlackboardAction));
        self.register(Box::new(GetBlackboardAction));
    }
    
    pub fn register(&mut self, action: Box<dyn ActionNode>) {
        self.actions.insert(action.name().to_string(), Arc::from(action));
    }
    
    pub fn get(&self, name: &str) -> Option<Arc<dyn ActionNode>> {
        self.actions.get(name).cloned()
    }
    
    pub fn contains(&self, name: &str) -> bool {
        self.actions.contains_key(name)
    }
    
    pub fn list(&self) -> Vec<String> {
        self.actions.keys().cloned().collect()
    }
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Wait action: pauses for specified milliseconds
pub struct WaitAction;

#[async_trait]
impl ActionNode for WaitAction {
    fn name(&self) -> &str {
        "wait"
    }
    
    async fn execute(&self, _ctx: &mut ExecutionContext, params: &HashMap<String, serde_json::Value>) -> NodeStatus {
        let ms = params.get("ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(100);
        
        tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
        NodeStatus::Success
    }
}

/// Log action: logs a message
pub struct LogAction;

#[async_trait]
impl ActionNode for LogAction {
    fn name(&self) -> &str {
        "log"
    }
    
    async fn execute(&self, _ctx: &mut ExecutionContext, params: &HashMap<String, serde_json::Value>) -> NodeStatus {
        let level = params.get("level")
            .and_then(|v| v.as_str())
            .unwrap_or("info");
        
        let message = params.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        match level {
            "error" => tracing::error!(message),
            "warn" => tracing::warn!(message),
            "debug" => tracing::debug!(message),
            _ => tracing::info!(message),
        }
        
        NodeStatus::Success
    }
}

/// SetBlackboard action: sets a value on the blackboard
pub struct SetBlackboardAction;

#[async_trait]
impl ActionNode for SetBlackboardAction {
    fn name(&self) -> &str {
        "set_blackboard"
    }
    
    async fn execute(&self, ctx: &mut ExecutionContext, params: &HashMap<String, serde_json::Value>) -> NodeStatus {
        let key = params.get("key")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let value = params.get("value").cloned().unwrap_or(serde_json::Value::Null);
        
        if !key.is_empty() {
            ctx.blackboard.set(key, value);
        }
        
        NodeStatus::Success
    }
}

/// GetBlackboard action: gets a value from the blackboard (for debugging)
pub struct GetBlackboardAction;

#[async_trait]
impl ActionNode for GetBlackboardAction {
    fn name(&self) -> &str {
        "get_blackboard"
    }
    
    async fn execute(&self, ctx: &mut ExecutionContext, params: &HashMap<String, serde_json::Value>) -> NodeStatus {
        let key = params.get("key")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        if !key.is_empty() {
            let value = ctx.blackboard.get(key);
            tracing::debug!(key, ?value, "Blackboard value");
        }
        
        NodeStatus::Success
    }
}

/// Action node executor that handles both built-in and plugin actions
pub struct ActionExecutor {
    registry: ActionRegistry,
    plugin_executor: Option<Arc<crate::wasm::PluginExecutor>>,
}

impl ActionExecutor {
    pub fn new(registry: ActionRegistry) -> Self {
        Self {
            registry,
            plugin_executor: None,
        }
    }
    
    pub fn with_plugin_executor(mut self, executor: Arc<crate::wasm::PluginExecutor>) -> Self {
        self.plugin_executor = Some(executor);
        self
    }
    
    pub async fn execute_action(
        &self,
        ctx: &mut ExecutionContext,
        action_name: &str,
        plugin_name: Option<&str>,
        params: &HashMap<String, serde_json::Value>,
    ) -> NodeStatus {
        // Try plugin first if specified
        if let Some(plugin) = plugin_name {
            if let Some(executor) = &self.plugin_executor {
                if let Ok(status) = executor.execute_action(plugin, action_name, &ctx.blackboard, params).await {
                    return status;
                }
            }
        }
        
        // Fall back to built-in registry
        if let Some(action) = self.registry.get(action_name) {
            action.execute(ctx, params).await
        } else {
            tracing::warn!(action = action_name, "Action not found");
            NodeStatus::Error
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bt::Blackboard;
    use std::collections::HashMap;
    
    #[tokio::test]
    async fn test_wait_action() {
        let action = WaitAction;
        let mut ctx = ExecutionContext::new(Blackboard::new());
        let mut params = HashMap::new();
        params.insert("ms".to_string(), serde_json::Value::Number(10.into()));
        
        let start = std::time::Instant::now();
        let status = action.execute(&mut ctx, &params).await;
        let elapsed = start.elapsed();
        
        assert_eq!(status, NodeStatus::Success);
        assert!(elapsed >= std::time::Duration::from_millis(10));
    }
    
    #[tokio::test]
    async fn test_set_blackboard_action() {
        let action = SetBlackboardAction;
        let mut ctx = ExecutionContext::new(Blackboard::new());
        let mut params = HashMap::new();
        params.insert("key".to_string(), serde_json::Value::String("test_key".to_string()));
        params.insert("value".to_string(), serde_json::Value::String("test_value".to_string()));
        
        let status = action.execute(&mut ctx, &params).await;
        
        assert_eq!(status, NodeStatus::Success);
        assert_eq!(ctx.blackboard.get_string("test_key"), Some("test_value".to_string()));
    }
    
    #[test]
    fn test_action_registry() {
        let registry = ActionRegistry::new();
        assert!(registry.contains("wait"));
        assert!(registry.contains("log"));
        assert!(registry.contains("set_blackboard"));
        assert!(registry.contains("get_blackboard"));
    }
}