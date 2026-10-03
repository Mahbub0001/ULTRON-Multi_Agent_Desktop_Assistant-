//! Behavior Tree core types and node definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for a node in the behavior tree
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Execution status of a behavior tree node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    /// Node is waiting to be executed
    Pending,
    /// Node is currently executing
    Running,
    /// Node completed successfully
    Success,
    /// Node failed
    Failure,
    /// Node was skipped (e.g., due to decorator)
    Skipped,
    /// Node encountered an error
    Error,
    Stopped,
}

impl NodeStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, NodeStatus::Success | NodeStatus::Failure | NodeStatus::Error | NodeStatus::Skipped)
    }
    
    pub fn is_running(&self) -> bool {
        matches!(self, NodeStatus::Running)
    }
}

/// Type of a behavior tree node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    // Composite nodes
    Sequence,
    Selector,
    Parallel,
    // Decorator nodes
    Inverter,
    Repeater,
    UntilSuccess,
    UntilFailure,
    Retry,
    Timeout,
    // Leaf nodes
    Action,
    Condition,
    // Special
    Plugin,
}

/// Configuration for a node
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "config")]
pub enum NodeConfig {
    // Composite nodes
    Sequence { children: Vec<NodeId> },
    Selector { children: Vec<NodeId> },
    Parallel { 
        children: Vec<NodeId>,
        success_threshold: usize,
        failure_threshold: usize,
    },
    // Decorator nodes
    Inverter { child: NodeId },
    Repeater { child: NodeId, count: usize },
    UntilSuccess { child: NodeId },
    UntilFailure { child: NodeId },
    Retry { child: NodeId, max_attempts: usize },
    Timeout { child: NodeId, timeout_ms: u64 },
    // Leaf nodes
    Action { 
        name: String,
        plugin: Option<String>,
        parameters: HashMap<String, serde_json::Value>,
    },
    Condition { 
        name: String,
        plugin: Option<String>,
        parameters: HashMap<String, serde_json::Value>,
        negate: bool,
    },
    // Plugin node (WASM)
    Plugin {
        plugin_name: String,
        export_name: String,
        parameters: HashMap<String, serde_json::Value>,
    },
}

/// A node in the behavior tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub name: String,
    pub node_type: NodeType,
    pub config: NodeConfig,
    pub description: Option<String>,
}

impl Node {
    pub fn new_sequence(name: impl Into<String>, children: Vec<NodeId>) -> Self {
        Self {
            id: NodeId::new(),
            name: name.into(),
            node_type: NodeType::Sequence,
            config: NodeConfig::Sequence { children },
            description: None,
        }
    }
    
    pub fn new_selector(name: impl Into<String>, children: Vec<NodeId>) -> Self {
        Self {
            id: NodeId::new(),
            name: name.into(),
            node_type: NodeType::Selector,
            config: NodeConfig::Selector { children },
            description: None,
        }
    }
    
    pub fn new_parallel(
        name: impl Into<String>, 
        children: Vec<NodeId>,
        success_threshold: usize,
        failure_threshold: usize,
    ) -> Self {
        Self {
            id: NodeId::new(),
            name: name.into(),
            node_type: NodeType::Parallel,
            config: NodeConfig::Parallel { 
                children,
                success_threshold,
                failure_threshold,
            },
            description: None,
        }
    }
    
    pub fn new_inverter(name: impl Into<String>, child: NodeId) -> Self {
        Self {
            id: NodeId::new(),
            name: name.into(),
            node_type: NodeType::Inverter,
            config: NodeConfig::Inverter { child },
            description: None,
        }
    }
    
    pub fn new_repeater(name: impl Into<String>, child: NodeId, count: usize) -> Self {
        Self {
            id: NodeId::new(),
            name: name.into(),
            node_type: NodeType::Repeater,
            config: NodeConfig::Repeater { child, count },
            description: None,
        }
    }
    
    pub fn new_until_success(name: impl Into<String>, child: NodeId) -> Self {
        Self {
            id: NodeId::new(),
            name: name.into(),
            node_type: NodeType::UntilSuccess,
            config: NodeConfig::UntilSuccess { child },
            description: None,
        }
    }
    
    pub fn new_until_failure(name: impl Into<String>, child: NodeId) -> Self {
        Self {
            id: NodeId::new(),
            name: name.into(),
            node_type: NodeType::UntilFailure,
            config: NodeConfig::UntilFailure { child },
            description: None,
        }
    }
    
    pub fn new_retry(name: impl Into<String>, child: NodeId, max_attempts: usize) -> Self {
        Self {
            id: NodeId::new(),
            name: name.into(),
            node_type: NodeType::Retry,
            config: NodeConfig::Retry { child, max_attempts },
            description: None,
        }
    }
    
    pub fn new_timeout(name: impl Into<String>, child: NodeId, timeout_ms: u64) -> Self {
        Self {
            id: NodeId::new(),
            name: name.into(),
            node_type: NodeType::Timeout,
            config: NodeConfig::Timeout { child, timeout_ms },
            description: None,
        }
    }
    
    pub fn new_action(
        name: impl Into<String>,
        action_name: impl Into<String>,
        plugin: Option<String>,
        parameters: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            id: NodeId::new(),
            name: name.into(),
            node_type: NodeType::Action,
            config: NodeConfig::Action {
                name: action_name.into(),
                plugin,
                parameters,
            },
            description: None,
        }
    }
    
    pub fn new_condition(
        name: impl Into<String>,
        condition_name: impl Into<String>,
        plugin: Option<String>,
        parameters: HashMap<String, serde_json::Value>,
        negate: bool,
    ) -> Self {
        Self {
            id: NodeId::new(),
            name: name.into(),
            node_type: NodeType::Condition,
            config: NodeConfig::Condition {
                name: condition_name.into(),
                plugin,
                parameters,
                negate,
            },
            description: None,
        }
    }
    
    pub fn new_plugin(
        name: impl Into<String>,
        plugin_name: impl Into<String>,
        export_name: impl Into<String>,
        parameters: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            id: NodeId::new(),
            name: name.into(),
            node_type: NodeType::Plugin,
            config: NodeConfig::Plugin {
                plugin_name: plugin_name.into(),
                export_name: export_name.into(),
                parameters,
            },
            description: None,
        }
    }
    
    pub fn children(&self) -> Vec<NodeId> {
        match &self.config {
            NodeConfig::Sequence { children } => children.clone(),
            NodeConfig::Selector { children } => children.clone(),
            NodeConfig::Parallel { children, .. } => children.clone(),
            NodeConfig::Inverter { child } => vec![*child],
            NodeConfig::Repeater { child, .. } => vec![*child],
            NodeConfig::UntilSuccess { child } => vec![*child],
            NodeConfig::UntilFailure { child } => vec![*child],
            NodeConfig::Retry { child, .. } => vec![*child],
            NodeConfig::Timeout { child, .. } => vec![*child],
            NodeConfig::Action { .. } | NodeConfig::Condition { .. } | NodeConfig::Plugin { .. } => vec![],
        }
    }
}

/// Complete behavior tree definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorTree {
    pub id: String,
    pub name: String,
    pub version: String,
    pub root: NodeId,
    pub nodes: HashMap<NodeId, Node>,
    pub blackboard: Blackboard,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl BehaviorTree {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            version: "1.0.0".to_string(),
            root: NodeId::new(),
            nodes: HashMap::new(),
            blackboard: Blackboard::new(),
            metadata: HashMap::new(),
        }
    }
    
    pub fn add_node(&mut self, node: Node) -> NodeId {
        let id = node.id;
        self.nodes.insert(id, node);
        id
    }
    
    pub fn set_root(&mut self, root: NodeId) {
        self.root = root;
    }
    
    pub fn get_node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }
    
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(&id)
    }
    
    pub fn validate(&self) -> Result<(), String> {
        if !self.nodes.contains_key(&self.root) {
            return Err("Root node not found in tree".to_string());
        }
        
        // Check for cycles
        let mut visited = std::collections::HashSet::new();
        let mut stack = std::collections::HashSet::new();
        
        fn check_cycles(
            tree: &BehaviorTree,
            node_id: NodeId,
            visited: &mut std::collections::HashSet<NodeId>,
            stack: &mut std::collections::HashSet<NodeId>,
        ) -> Result<(), String> {
            if stack.contains(&node_id) {
                return Err(format!("Cycle detected at node {}", node_id));
            }
            if visited.contains(&node_id) {
                return Ok(());
            }
            
            stack.insert(node_id);
            
            if let Some(node) = tree.nodes.get(&node_id) {
                for child in node.children() {
                    check_cycles(tree, child, visited, stack)?;
                }
            }
            
            stack.remove(&node_id);
            visited.insert(node_id);
            Ok(())
        }
        
        check_cycles(self, self.root, &mut visited, &mut stack)?;
        
        // Check all referenced nodes exist
        for (id, node) in &self.nodes {
            for child in node.children() {
                if !self.nodes.contains_key(&child) {
                    return Err(format!("Node {} references non-existent child {}", id, child));
                }
            }
        }
        
        Ok(())
    }
    
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
    
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
    
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
    
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }
}

pub use super::blackboard::Blackboard;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_node_creation() {
        let action = Node::new_action("Test Action", "move_to", None, HashMap::new());
        assert_eq!(action.node_type, NodeType::Action);
        
        let condition = Node::new_condition("Test Condition", "is_visible", None, HashMap::new(), false);
        assert_eq!(condition.node_type, NodeType::Condition);
    }
    
    #[test]
    fn test_blackboard() {
        let mut bb = Blackboard::new();
        bb.set("key1", "value1");
        bb.set("key2", 42);
        
        assert_eq!(bb.get_string("key1"), Some("value1".to_string()));
        assert_eq!(bb.get_int("key2"), Some(42));
        assert!(bb.has("key1"));
        assert!(!bb.has("key3"));
    }
    
    #[test]
    fn test_blackboard_parent() {
        let mut parent = Blackboard::new();
        parent.set("parent_key", "parent_value");
        
        let mut child = Blackboard::with_parent(parent);
        child.set("child_key", "child_value");
        
        assert_eq!(child.get_string("parent_key"), Some("parent_value".to_string()));
        assert_eq!(child.get_string("child_key"), Some("child_value".to_string()));
    }
    
    #[test]
    fn test_tree_validation() {
        let mut tree = BehaviorTree::new("Test Tree");
        
        let action1 = Node::new_action("Action 1", "action1", None, HashMap::new());
        let action2 = Node::new_action("Action 2", "action2", None, HashMap::new());
        let sequence = Node::new_sequence("Sequence", vec![action1.id, action2.id]);
        
        tree.add_node(action1);
        tree.add_node(action2);
        tree.add_node(sequence);
        tree.set_root(sequence.id);
        
        assert!(tree.validate().is_ok());
    }
    
    #[test]
    fn test_tree_cycle_detection() {
        let mut tree = BehaviorTree::new("Test Tree");
        
        let mut node1 = Node::new_sequence("Node 1", vec![]);
        let mut node2 = Node::new_sequence("Node 2", vec![]);
        
        // Create a cycle manually
        node1.config = NodeConfig::Sequence { children: vec![node2.id] };
        node2.config = NodeConfig::Sequence { children: vec![node1.id] };
        
        tree.add_node(node1);
        tree.add_node(node2);
        tree.set_root(node1.id);
        
        assert!(tree.validate().is_err());
    }
}