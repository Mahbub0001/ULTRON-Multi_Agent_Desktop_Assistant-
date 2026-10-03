//! Condition nodes: evaluatable conditions that return success/failure

use crate::bt::{NodeId, NodeStatus, Blackboard, ExecutionContext};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Trait for condition node evaluation
#[async_trait]
pub trait ConditionNode: Send + Sync {
    /// Evaluate the condition
    async fn evaluate(&self, ctx: &mut ExecutionContext, params: &HashMap<String, serde_json::Value>) -> bool;
    
    /// Get the condition name
    fn name(&self) -> &str;
    
    /// Get parameter schema (optional)
    fn parameter_schema(&self) -> Option<serde_json::Value> {
        None
    }
}

/// Built-in condition registry
pub struct ConditionRegistry {
    conditions: HashMap<String, Arc<dyn ConditionNode>>,
}

impl ConditionRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            conditions: HashMap::new(),
        };
        registry.register_builtins();
        registry
    }
    
    fn register_builtins(&mut self) {
        self.register(Box::new(BlackboardCondition));
        self.register(Box::new(CompareCondition));
        self.register(Box::new(AlwaysTrueCondition));
        self.register(Box::new(AlwaysFalseCondition));
    }
    
    pub fn register(&mut self, condition: Box<dyn ConditionNode>) {
        self.conditions.insert(condition.name().to_string(), Arc::from(condition));
    }
    
    pub fn get(&self, name: &str) -> Option<Arc<dyn ConditionNode>> {
        self.conditions.get(name).cloned()
    }
    
    pub fn contains(&self, name: &str) -> bool {
        self.conditions.contains_key(name)
    }
    
    pub fn list(&self) -> Vec<String> {
        self.conditions.keys().cloned().collect()
    }
}

impl Default for ConditionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Blackboard condition: checks if a blackboard key exists and optionally matches a value
pub struct BlackboardCondition;

#[async_trait]
impl ConditionNode for BlackboardCondition {
    fn name(&self) -> &str {
        "blackboard_check"
    }
    
    async fn evaluate(&self, ctx: &mut ExecutionContext, params: &HashMap<String, serde_json::Value>) -> bool {
        let key = match params.get("key").and_then(|v| v.as_str()) {
            Some(k) => k,
            None => return false,
        };
        
        // Check existence
        if !ctx.blackboard.has(key) {
            return false;
        }
        
        // If expected value provided, compare
        if let Some(expected) = params.get("value") {
            let actual = ctx.blackboard.get(key);
            return actual.as_ref() == Some(expected);
        }
        
        true
    }
}

/// Compare condition: compares two values
pub struct CompareCondition;

#[async_trait]
impl ConditionNode for CompareCondition {
    fn name(&self) -> &str {
        "compare"
    }
    
    async fn evaluate(&self, ctx: &mut ExecutionContext, params: &HashMap<String, serde_json::Value>) -> bool {
        let left = params.get("left");
        let right = params.get("right");
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("eq");
        
        let (left_val, right_val) = match (left, right) {
            (Some(l), Some(r)) => (l, r),
            _ => return false,
        };
        
        match op {
            "eq" => left_val == right_val,
            "ne" => left_val != right_val,
            "lt" => compare_numbers(left_val, right_val, |a, b| a < b),
            "le" => compare_numbers(left_val, right_val, |a, b| a <= b),
            "gt" => compare_numbers(left_val, right_val, |a, b| a > b),
            "ge" => compare_numbers(left_val, right_val, |a, b| a >= b),
            "contains" => {
                if let Some(arr) = left_val.as_array() {
                    arr.contains(right_val)
                } else if let (Some(str), Some(substr)) = (left_val.as_str(), right_val.as_str()) {
                    str.contains(substr)
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

fn compare_numbers(left: &serde_json::Value, right: &serde_json::Value, op: impl FnOnce(f64, f64) -> bool) -> bool {
    match (left.as_f64(), right.as_f64()) {
        (Some(l), Some(r)) => op(l, r),
        _ => false,
    }
}

/// Always true condition (for testing)
pub struct AlwaysTrueCondition;

#[async_trait]
impl ConditionNode for AlwaysTrueCondition {
    fn name(&self) -> &str {
        "always_true"
    }
    
    async fn evaluate(&self, _ctx: &mut ExecutionContext, _params: &HashMap<String, serde_json::Value>) -> bool {
        true
    }
}

/// Always false condition (for testing)
pub struct AlwaysFalseCondition;

#[async_trait]
impl ConditionNode for AlwaysFalseCondition {
    fn name(&self) -> &str {
        "always_false"
    }
    
    async fn evaluate(&self, _ctx: &mut ExecutionContext, _params: &HashMap<String, serde_json::Value>) -> bool {
        false
    }
}

/// Condition node executor
pub struct ConditionExecutor {
    registry: ConditionRegistry,
    plugin_executor: Option<Arc<crate::wasm::PluginExecutor>>,
}

impl ConditionExecutor {
    pub fn new(registry: ConditionRegistry) -> Self {
        Self {
            registry,
            plugin_executor: None,
        }
    }
    
    pub fn with_plugin_executor(mut self, executor: Arc<crate::wasm::PluginExecutor>) -> Self {
        self.plugin_executor = Some(executor);
        self
    }
    
    pub async fn evaluate_condition(
        &self,
        ctx: &mut ExecutionContext,
        condition_name: &str,
        plugin_name: Option<&str>,
        params: &HashMap<String, serde_json::Value>,
        negate: bool,
    ) -> NodeStatus {
        // Try plugin first if specified
        if let Some(plugin) = plugin_name {
            if let Some(executor) = &self.plugin_executor {
                if let Ok(result) = executor.evaluate_condition(plugin, condition_name, &ctx.blackboard, params).await {
                    return if negate { 
                        if result { NodeStatus::Failure } else { NodeStatus::Success }
                    } else {
                        if result { NodeStatus::Success } else { NodeStatus::Failure }
                    };
                }
            }
        }
        
        // Fall back to built-in registry
        if let Some(condition) = self.registry.get(condition_name) {
            let result = condition.evaluate(ctx, params).await;
            if negate {
                if result { NodeStatus::Failure } else { NodeStatus::Success }
            } else {
                if result { NodeStatus::Success } else { NodeStatus::Failure }
            }
        } else {
            tracing::warn!(condition = condition_name, "Condition not found");
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
    async fn test_blackboard_condition() {
        let condition = BlackboardCondition;
        let mut ctx = ExecutionContext::new(Blackboard::new());
        ctx.blackboard.set("test_key", "test_value");
        
        let mut params = HashMap::new();
        params.insert("key".to_string(), serde_json::Value::String("test_key".to_string()));
        
        let result = condition.evaluate(&mut ctx, &params).await;
        assert!(result);
        
        // Test with value match
        params.insert("value".to_string(), serde_json::Value::String("test_value".to_string()));
        let result = condition.evaluate(&mut ctx, &params).await;
        assert!(result);
        
        // Test with wrong value
        params.insert("value".to_string(), serde_json::Value::String("wrong".to_string()));
        let result = condition.evaluate(&mut ctx, &params).await;
        assert!(!result);
    }
    
    #[tokio::test]
    async fn test_compare_condition() {
        let condition = CompareCondition;
        let mut ctx = ExecutionContext::new(Blackboard::new());
        
        let mut params = HashMap::new();
        params.insert("left".to_string(), serde_json::Value::Number(10.into()));
        params.insert("right".to_string(), serde_json::Value::Number(5.into()));
        params.insert("op".to_string(), serde_json::Value::String("gt".to_string()));
        
        let result = condition.evaluate(&mut ctx, &params).await;
        assert!(result);
        
        params.insert("op".to_string(), serde_json::Value::String("lt".to_string()));
        let result = condition.evaluate(&mut ctx, &params).await;
        assert!(!result);
    }
    
    #[test]
    fn test_condition_registry() {
        let registry = ConditionRegistry::new();
        assert!(registry.contains("blackboard_check"));
        assert!(registry.contains("compare"));
        assert!(registry.contains("always_true"));
        assert!(registry.contains("always_false"));
    }
}