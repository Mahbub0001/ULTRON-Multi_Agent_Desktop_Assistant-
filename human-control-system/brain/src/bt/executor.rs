//! Behavior Tree Executor

use crate::bt::{
    Node, NodeId, NodeType, NodeStatus, NodeConfig, BehaviorTree, Blackboard,
    composite::{CompositeNode, SequenceNode, SelectorNode, ParallelNode, create_composite},
    decorator::{DecoratorNode, DecoratorState, create_decorator},
    action::{ActionExecutor, ActionRegistry},
    condition::{ConditionExecutor, ConditionRegistry},
};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, info, warn, error};
use uuid::Uuid;

/// Configuration for the executor
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    pub max_depth: usize,
    pub max_ticks: Option<u64>,
    pub tick_timeout_ms: u64,
    pub enable_visualization: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_depth: 100,
            max_ticks: None,
            tick_timeout_ms: 1000,
            enable_visualization: true,
        }
    }
}

/// Execution context for a single tree execution
pub struct ExecutionContext {
    nodes: HashMap<NodeId, Node>,
    pub plugin_executor: Option<Arc<crate::wasm::PluginExecutor>>,
    pub tree_id: String,
    pub blackboard: Blackboard,
    pub node_states: HashMap<NodeId, NodeStatus>,
    pub decorator_states: HashMap<NodeId, DecoratorState>,
    pub current_depth: usize,
    pub tick_count: u64,
    pub start_time: std::time::Instant,
    pub config: ExecutorConfig,
    pub action_executor: Arc<ActionExecutor>,
    pub condition_executor: Arc<ConditionExecutor>,
    pub visualization_data: Option<VisualizationData>,
}

impl ExecutionContext {
    pub fn new(blackboard: Blackboard) -> Self {
        let config = ExecutorConfig::default();
        let action_executor = Arc::new(ActionExecutor::new(ActionRegistry::new()));
        let condition_executor = Arc::new(ConditionExecutor::new(ConditionRegistry::new()));
        
        Self {
            nodes: HashMap::new(),
            plugin_executor: None,
            tree_id: Uuid::new_v4().to_string(),
            blackboard,
            node_states: HashMap::new(),
            decorator_states: HashMap::new(),
            current_depth: 0,
            tick_count: 0,
            start_time: std::time::Instant::now(),
            config: config.clone(),
            action_executor,
            condition_executor,
            visualization_data: if config.enable_visualization { Some(VisualizationData::new()) } else { None },
        }
    }
    
    pub fn with_config(mut self, config: ExecutorConfig) -> Self {
        self.config = config;
        self
    }
    
    pub fn with_action_executor(mut self, executor: Arc<ActionExecutor>) -> Self {
        self.action_executor = executor;
        self
    }
    
    pub fn with_condition_executor(mut self, executor: Arc<ConditionExecutor>) -> Self {
        self.condition_executor = executor;
        self
    }
    
    pub fn tick_node(&mut self, node: NodeId) -> std::pin::Pin<Box<dyn std::future::Future<Output = NodeStatus> + Send + '_>> {
        Box::pin(Executor::tick_node_recursive(self, node))
    }

    pub fn set_tree_id(&mut self, tree_id: String) {
        self.tree_id = tree_id;
    }
    
    pub fn get_node_status(&self, node_id: NodeId) -> NodeStatus {
        self.node_states.get(&node_id).copied().unwrap_or(NodeStatus::Pending)
    }
    
    pub fn set_node_status(&mut self, node_id: NodeId, status: NodeStatus) {
        self.node_states.insert(node_id, status);
        
        if let Some(viz) = &mut self.visualization_data {
            viz.record_node_status(node_id, status);
        }
    }
    
    pub fn get_decorator_state(&mut self, node_id: NodeId) -> &mut DecoratorState {
        self.decorator_states.entry(node_id).or_default()
    }
    
    pub fn increment_tick(&mut self) {
        self.tick_count += 1;
    }
    
    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }
    
    pub fn check_timeout(&self) -> bool {
        self.elapsed_ms() > self.config.tick_timeout_ms
    }
    
    pub fn check_max_ticks(&self) -> bool {
        if let Some(max) = self.config.max_ticks {
            self.tick_count >= max
        } else {
            false
        }
    }
}

/// Visualization data for debugging
# [derive(Debug, Clone, Default)]
pub struct VisualizationData {
    pub node_statuses: HashMap<NodeId, Vec<(u64, NodeStatus)>>, // tick -> status
    pub blackboard_snapshots: Vec<(u64, HashMap<String, serde_json::Value>)>,
}

impl VisualizationData {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn record_node_status(&mut self, node_id: NodeId, status: NodeStatus) {
        let tick = 0; // Would be set by executor
        self.node_statuses.entry(node_id).or_default().push((tick, status));
    }
    
    pub fn record_blackboard(&mut self, tick: u64, blackboard: &Blackboard) {
        self.blackboard_snapshots.push((tick, blackboard.snapshot()));
    }
}

/// Result of a single tick
#[derive(Debug, Clone)]
pub struct TickResult {
    pub status: NodeStatus,
    pub blackboard: Blackboard,
    pub tick_count: u64,
    pub elapsed_ms: u64,
    pub visualization: Option<VisualizationData>,
}

/// Behavior Tree Executor
pub struct Executor {
    tree: BehaviorTree,
    context: Option<ExecutionContext>,
    plugin_executor: Option<Arc<crate::wasm::PluginExecutor>>,
    config: ExecutorConfig,
    action_executor: Arc<ActionExecutor>,
    condition_executor: Arc<ConditionExecutor>,
    running: Arc<RwLock<bool>>,
}

impl Executor {
    pub fn new(tree: BehaviorTree) -> Self {
        let config = ExecutorConfig::default();
        let action_executor = Arc::new(ActionExecutor::new(ActionRegistry::new()));
        let condition_executor = Arc::new(ConditionExecutor::new(ConditionRegistry::new()));
        
        Self {
            tree,
            context: None,
            plugin_executor: None,
            config,
            action_executor,
            condition_executor,
            running: Arc::new(RwLock::new(false)),
        }
    }
    
    pub fn with_config(mut self, config: ExecutorConfig) -> Self {
        self.config = config;
        self
    }
    
    pub fn with_action_executor(mut self, executor: Arc<ActionExecutor>) -> Self {
        self.action_executor = executor;
        self
    }
    
    pub fn with_condition_executor(mut self, executor: Arc<ConditionExecutor>) -> Self {
        self.condition_executor = executor;
        self
    }
    
    pub fn tree(&self) -> &BehaviorTree {
        &self.tree
    }
    
    pub fn tree_mut(&mut self) -> &mut BehaviorTree {
        &mut self.tree
    }
    
    pub fn is_running(&self) -> bool {
        *self.running.read()
    }
    
    /// Execute a single tick of the behavior tree
    pub async fn tick(&mut self) -> TickResult {
        *self.running.write() = true;
        let mut ctx = self.context.take().unwrap_or_else(|| {
            ExecutionContext::new(self.tree.blackboard.clone())
                .with_config(self.config.clone())
                .with_action_executor(self.action_executor.clone())
                .with_condition_executor(self.condition_executor.clone())
        });
        ctx.nodes = self.tree.nodes.clone();
        ctx.plugin_executor = self.plugin_executor.clone();
        ctx.set_tree_id(self.tree.id.clone());
        ctx.increment_tick();
        let status = tokio::time::timeout(
            std::time::Duration::from_millis(self.config.tick_timeout_ms),
            ctx.tick_node(self.tree.root),
        ).await.unwrap_or(NodeStatus::Error);
        let result = TickResult {
            status,
            blackboard: ctx.blackboard.clone(),
            tick_count: ctx.tick_count,
            elapsed_ms: ctx.elapsed_ms(),
            visualization: ctx.visualization_data.clone(),
        };
        self.tree.blackboard = result.blackboard.clone();
        self.context = Some(ctx);
        *self.running.write() = false;
        result
    }

    pub fn with_plugin_executor(mut self, executor: Arc<crate::wasm::PluginExecutor>) -> Self {
        self.plugin_executor = Some(executor);
        self
    }

    /// Execute tree until completion or max ticks
    pub async fn execute(&mut self) -> TickResult {
        let mut final_result = None;
        
        loop {
            let result = self.tick().await;
            final_result = Some(result.clone());
            
            match result.status {
                NodeStatus::Success | NodeStatus::Failure | NodeStatus::Error => break,
                NodeStatus::Running => {
                    if self.config.max_ticks.is_some_and(|max| result.tick_count >= max) || result.elapsed_ms >= self.config.tick_timeout_ms {
                        break;
                    }
                    // Small yield to prevent busy loop
                    tokio::task::yield_now().await;
                }
                _ => break,
            }
        }
        
        final_result.unwrap_or_else(|| TickResult {
            status: NodeStatus::Error,
            blackboard: Blackboard::new(),
            tick_count: 0,
            elapsed_ms: 0,
            visualization: None,
        })
    }
    
    /// Stop execution
    pub fn stop(&self) {
        let mut running = self.running.write();
        *running = false;
    }
    
    /// Recursive node execution
    async fn tick_node_recursive(ctx: &mut ExecutionContext, node_id: NodeId) -> NodeStatus {
        if ctx.current_depth >= ctx.config.max_depth {
            error!(node_id = %node_id, "Max depth exceeded");
            return NodeStatus::Error;
        }
        
        ctx.current_depth += 1;
        let result = Self::execute_node(ctx, node_id).await;
        ctx.current_depth -= 1;
        
        ctx.set_node_status(node_id, result);
        result
    }
    
    async fn execute_node(ctx: &mut ExecutionContext, node_id: NodeId) -> NodeStatus {
        let node = match ctx.nodes.get(&node_id) {
            Some(n) => n.clone(),
            None => {
                error!(node_id = %node_id, "Node not found");
                return NodeStatus::Error;
            }
        };
        
        debug!(tree_id = %ctx.tree_id, node_id = %node_id, node_name = %node.name, node_type = ?node.node_type, "Executing node");
        
        let result = match node.node_type {
            NodeType::Sequence | NodeType::Selector | NodeType::Parallel => {
                Self::execute_composite(ctx, &node).await
            }
            NodeType::Inverter | NodeType::Repeater | NodeType::UntilSuccess 
                | NodeType::UntilFailure | NodeType::Retry | NodeType::Timeout => {
                Self::execute_decorator(ctx, &node).await
            }
            NodeType::Action => {
                Self::execute_action(ctx, &node).await
            }
            NodeType::Condition => {
                Self::execute_condition(ctx, &node).await
            }
            NodeType::Plugin => {
                Self::execute_plugin(ctx, &node).await
            }
        };
        
        result
    }
    
    async fn execute_composite(ctx: &mut ExecutionContext, node: &Node) -> NodeStatus {
        let children = match &node.config {
            NodeConfig::Sequence { children } => children,
            NodeConfig::Selector { children } => children,
            NodeConfig::Parallel { children, .. } => children,
            _ => return NodeStatus::Error,
        };
        
        let composite: Box<dyn CompositeNode> = match node.node_type {
            NodeType::Sequence => Box::new(SequenceNode),
            NodeType::Selector => Box::new(SelectorNode),
            NodeType::Parallel => {
                if let NodeConfig::Parallel { success_threshold, failure_threshold, .. } = &node.config {
                    Box::new(ParallelNode {
                        success_threshold: *success_threshold,
                        failure_threshold: *failure_threshold,
                    })
                } else {
                    return NodeStatus::Error;
                }
            }
            _ => return NodeStatus::Error,
        };
        
        composite.tick(ctx, children).await
    }
    
    async fn execute_decorator(ctx: &mut ExecutionContext, node: &Node) -> NodeStatus {
        let (child, decorator) = match &node.config {
            NodeConfig::Inverter { child } => (*child, create_decorator(NodeType::Inverter, &node.config)),
            NodeConfig::Repeater { child, .. } => (*child, create_decorator(NodeType::Repeater, &node.config)),
            NodeConfig::UntilSuccess { child } => (*child, create_decorator(NodeType::UntilSuccess, &node.config)),
            NodeConfig::UntilFailure { child } => (*child, create_decorator(NodeType::UntilFailure, &node.config)),
            NodeConfig::Retry { child, .. } => (*child, create_decorator(NodeType::Retry, &node.config)),
            NodeConfig::Timeout { child, .. } => (*child, create_decorator(NodeType::Timeout, &node.config)),
            _ => return NodeStatus::Error,
        };
        
        let mut decorator = match decorator {
            Some(d) => d,
            None => return NodeStatus::Error,
        };
        
        let mut state = ctx.get_decorator_state(node.id).clone();
        let result = decorator.tick(ctx, child, &mut state).await;
        
        // Update state
        *ctx.get_decorator_state(node.id) = state;
        
        result
    }
    
    async fn execute_action(ctx: &mut ExecutionContext, node: &Node) -> NodeStatus {
        let (action_name, plugin_name, params) = match &node.config {
            NodeConfig::Action { name, plugin, parameters } => (name.clone(), plugin.clone(), parameters.clone()),
            _ => return NodeStatus::Error,
        };
        
        ctx.action_executor.clone().execute_action(ctx, &action_name, plugin_name.as_deref(), &params).await
    }
    
    async fn execute_condition(ctx: &mut ExecutionContext, node: &Node) -> NodeStatus {
        let (condition_name, plugin_name, params, negate) = match &node.config {
            NodeConfig::Condition { name, plugin, parameters, negate } => (name.clone(), plugin.clone(), parameters.clone(), *negate),
            _ => return NodeStatus::Error,
        };
        
        ctx.condition_executor.clone().evaluate_condition(ctx, &condition_name, plugin_name.as_deref(), &params, negate).await
    }
    
    async fn execute_plugin(ctx: &mut ExecutionContext, node: &Node) -> NodeStatus {
        let (plugin_name, export_name, params) = match &node.config {
            NodeConfig::Plugin { plugin_name, export_name, parameters } => (plugin_name.clone(), export_name.clone(), parameters.clone()),
            _ => return NodeStatus::Error,
        };
        
        if let Some(executor) = &ctx.plugin_executor {
            match executor.execute_action(&plugin_name, &export_name, &ctx.blackboard, &params).await {
                Ok(status) => status,
                Err(e) => {
                    error!(plugin = %plugin_name, export = %export_name, error = %e, "Plugin execution failed");
                    NodeStatus::Error
                }
            }
        } else {
            warn!(plugin = %plugin_name, "No plugin executor available");
            NodeStatus::Error
        }
    }
    
    /// Export tree visualization as DOT format
    pub fn to_dot(&self) -> String {
        let mut dot = String::new();
        dot.push_str("digraph BehaviorTree {\n");
        dot.push_str("  rankdir=TB;\n");
        dot.push_str("  node [shape=box, style=filled];\n");
        
        for (id, node) in &self.tree.nodes {
            let label = format!("{} ({})", node.name, format!("{:?}", node.node_type).to_lowercase());
            let color = match node.node_type {
                NodeType::Sequence | NodeType::Selector | NodeType::Parallel => "lightblue",
                NodeType::Inverter | NodeType::Repeater | NodeType::UntilSuccess 
                    | NodeType::UntilFailure | NodeType::Retry | NodeType::Timeout => "lightyellow",
                NodeType::Action => "lightgreen",
                NodeType::Condition => "lightcoral",
                NodeType::Plugin => "plum",
            };
            dot.push_str(&format!("  \"{}\" [label=\"{}\", fillcolor=\"{}\"];\n", id, label, color));
        }
        
        for (id, node) in &self.tree.nodes {
            for child in node.children() {
                dot.push_str(&format!("  \"{}\" -> \"{}\";\n", id, child));
            }
        }
        
        dot.push_str("}\n");
        dot
    }
    
    /// Export tree visualization as JSON
    pub fn to_json_visualization(&self) -> serde_json::Value {
        fn node_to_json(tree: &BehaviorTree, node_id: NodeId) -> serde_json::Value {
            let node = tree.nodes.get(&node_id).unwrap();
            let mut json = serde_json::json!({
                "id": node_id.to_string(),
                "name": node.name,
                "type": format!("{:?}", node.node_type).to_lowercase(),
            });
            
            let children: Vec<_> = node.children().iter()
                .map(|c| node_to_json(tree, *c))
                .collect();
            
            if !children.is_empty() {
                json["children"] = serde_json::Value::Array(children);
            }
            
            json
        }
        
        node_to_json(&self.tree, self.tree.root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bt::{Node, NodeId, BehaviorTree};
    use std::collections::HashMap;
    
    fn create_simple_sequence() -> BehaviorTree {
        let mut tree = BehaviorTree::new("Test Sequence");
        
        let action1 = Node::new_action("Action 1", "always_success", None, HashMap::new());
        let action2 = Node::new_action("Action 2", "always_success", None, HashMap::new());
        
        let id1 = action1.id;
        let id2 = action2.id;
        
        let sequence = Node::new_sequence("Root Sequence", vec![id1, id2]);
        let root_id = sequence.id;
        
        tree.add_node(action1);
        tree.add_node(action2);
        tree.add_node(sequence);
        tree.set_root(root_id);
        
        tree
    }
    
    #[tokio::test]
    async fn test_executor_creation() {
        let tree = create_simple_sequence();
        let executor = Executor::new(tree);
        assert!(!executor.is_running());
    }
    
    #[tokio::test]
    async fn test_to_dot() {
        let tree = create_simple_sequence();
        let executor = Executor::new(tree);
        let dot = executor.to_dot();
        
        assert!(dot.contains("digraph BehaviorTree"));
        assert!(dot.contains("Root Sequence"));
        assert!(dot.contains("Action 1"));
        assert!(dot.contains("Action 2"));
    }
}