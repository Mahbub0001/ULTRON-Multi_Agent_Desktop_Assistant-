//! Decorator nodes: Inverter, Repeater, UntilSuccess, UntilFailure, Retry, Timeout

use crate::bt::{NodeId, NodeStatus, Blackboard, ExecutionContext};
use async_trait::async_trait;
use std::time::{Duration, Instant};
use std::sync::Arc;
use parking_lot::Mutex;

/// Trait for decorator node execution logic
#[async_trait]
pub trait DecoratorNode: Send + Sync {
    async fn tick(&self, ctx: &mut ExecutionContext, child: NodeId, state: &mut DecoratorState) -> NodeStatus;
}

/// State maintained by decorator nodes during execution
#[derive(Debug, Clone, Default)]
pub struct DecoratorState {
    pub attempt: usize,
    pub start_time: Option<Instant>,
    pub child_status: Option<NodeStatus>,
    pub custom: std::collections::HashMap<String, serde_json::Value>,
}

/// Inverter decorator: inverts child status (Success <-> Failure)
pub struct InverterNode;

#[async_trait]
impl DecoratorNode for InverterNode {
    async fn tick(&self, ctx: &mut ExecutionContext, child: NodeId, _state: &mut DecoratorState) -> NodeStatus {
        let status = ctx.tick_node(child).await;
        match status {
            NodeStatus::Success => NodeStatus::Failure,
            NodeStatus::Failure => NodeStatus::Success,
            NodeStatus::Running => NodeStatus::Running,
            NodeStatus::Error => NodeStatus::Error,
            NodeStatus::Skipped => NodeStatus::Skipped,
            NodeStatus::Pending => NodeStatus::Pending,
            NodeStatus::Stopped => NodeStatus::Stopped,
        }
    }
}

/// Repeater decorator: repeats child N times
pub struct RepeaterNode {
    pub count: usize,
}

#[async_trait]
impl DecoratorNode for RepeaterNode {
    async fn tick(&self, ctx: &mut ExecutionContext, child: NodeId, state: &mut DecoratorState) -> NodeStatus {
        if state.attempt >= self.count {
            return NodeStatus::Success;
        }
        
        let status = ctx.tick_node(child).await;
        
        match status {
            NodeStatus::Success | NodeStatus::Failure | NodeStatus::Error => {
                state.attempt += 1;
                if state.attempt >= self.count {
                    NodeStatus::Success
                } else {
                    NodeStatus::Running
                }
            }
            NodeStatus::Running => NodeStatus::Running,
            NodeStatus::Skipped => {
                state.attempt += 1;
                if state.attempt >= self.count {
                    NodeStatus::Success
                } else {
                    NodeStatus::Running
                }
            }
            NodeStatus::Pending => NodeStatus::Pending,
            NodeStatus::Stopped => NodeStatus::Stopped,
        }
    }
}

/// UntilSuccess decorator: repeats child until it succeeds
pub struct UntilSuccessNode;

#[async_trait]
impl DecoratorNode for UntilSuccessNode {
    async fn tick(&self, ctx: &mut ExecutionContext, child: NodeId, _state: &mut DecoratorState) -> NodeStatus {
        let status = ctx.tick_node(child).await;
        match status {
            NodeStatus::Success => NodeStatus::Success,
            NodeStatus::Running => NodeStatus::Running,
            NodeStatus::Failure | NodeStatus::Error => NodeStatus::Running, // Keep trying
            NodeStatus::Skipped => NodeStatus::Running,
            NodeStatus::Pending => NodeStatus::Pending,
            NodeStatus::Stopped => NodeStatus::Stopped,
        }
    }
}

/// UntilFailure decorator: repeats child until it fails
pub struct UntilFailureNode;

#[async_trait]
impl DecoratorNode for UntilFailureNode {
    async fn tick(&self, ctx: &mut ExecutionContext, child: NodeId, _state: &mut DecoratorState) -> NodeStatus {
        let status = ctx.tick_node(child).await;
        match status {
            NodeStatus::Failure => NodeStatus::Success, // Success when child fails
            NodeStatus::Running => NodeStatus::Running,
            NodeStatus::Success => NodeStatus::Running, // Keep trying
            NodeStatus::Error => NodeStatus::Error,
            NodeStatus::Skipped => NodeStatus::Running,
            NodeStatus::Pending => NodeStatus::Pending,
            NodeStatus::Stopped => NodeStatus::Stopped,
        }
    }
}

/// Retry decorator: retries child up to max_attempts times on failure
pub struct RetryNode {
    pub max_attempts: usize,
}

#[async_trait]
impl DecoratorNode for RetryNode {
    async fn tick(&self, ctx: &mut ExecutionContext, child: NodeId, state: &mut DecoratorState) -> NodeStatus {
        if state.attempt >= self.max_attempts {
            return NodeStatus::Failure;
        }
        
        let status = ctx.tick_node(child).await;
        
        match status {
            NodeStatus::Success => NodeStatus::Success,
            NodeStatus::Running => NodeStatus::Running,
            NodeStatus::Failure | NodeStatus::Error => {
                state.attempt += 1;
                if state.attempt >= self.max_attempts {
                    NodeStatus::Failure
                } else {
                    NodeStatus::Running
                }
            }
            NodeStatus::Skipped => {
                state.attempt += 1;
                if state.attempt >= self.max_attempts {
                    NodeStatus::Failure
                } else {
                    NodeStatus::Running
                }
            }
            NodeStatus::Pending => NodeStatus::Pending,
            NodeStatus::Stopped => NodeStatus::Stopped,
        }
    }
}

/// Timeout decorator: fails child if it runs longer than timeout_ms
pub struct TimeoutNode {
    pub timeout_ms: u64,
}

#[async_trait]
impl DecoratorNode for TimeoutNode {
    async fn tick(&self, ctx: &mut ExecutionContext, child: NodeId, state: &mut DecoratorState) -> NodeStatus {
        let now = Instant::now();
        
        if state.start_time.is_none() {
            state.start_time = Some(now);
        }
        
        let elapsed = now.duration_since(state.start_time.unwrap());
        
        if elapsed >= Duration::from_millis(self.timeout_ms) {
            state.start_time = None;
            return NodeStatus::Failure;
        }
        
        let status = ctx.tick_node(child).await;
        
        match status {
            NodeStatus::Success | NodeStatus::Failure | NodeStatus::Error => {
                state.start_time = None;
                status
            }
            NodeStatus::Running => NodeStatus::Running,
            NodeStatus::Skipped => {
                state.start_time = None;
                NodeStatus::Skipped
            }
            NodeStatus::Pending => NodeStatus::Pending,
            NodeStatus::Stopped => NodeStatus::Stopped,
        }
    }
}

/// Factory for creating decorator nodes
pub fn create_decorator(node_type: crate::bt::NodeType, config: &crate::bt::NodeConfig) -> Option<Box<dyn DecoratorNode>> {
    match node_type {
        crate::bt::NodeType::Inverter => Some(Box::new(InverterNode)),
        crate::bt::NodeType::Repeater => {
            if let crate::bt::NodeConfig::Repeater { count, .. } = config {
                Some(Box::new(RepeaterNode { count: *count }))
            } else { None }
        }
        crate::bt::NodeType::UntilSuccess => Some(Box::new(UntilSuccessNode)),
        crate::bt::NodeType::UntilFailure => Some(Box::new(UntilFailureNode)),
        crate::bt::NodeType::Retry => {
            if let crate::bt::NodeConfig::Retry { max_attempts, .. } = config {
                Some(Box::new(RetryNode { max_attempts: *max_attempts }))
            } else { None }
        }
        crate::bt::NodeType::Timeout => {
            if let crate::bt::NodeConfig::Timeout { timeout_ms, .. } = config {
                Some(Box::new(TimeoutNode { timeout_ms: *timeout_ms }))
            } else { None }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_decorator_state() {
        let mut state = DecoratorState::default();
        assert_eq!(state.attempt, 0);
        state.attempt = 5;
        assert_eq!(state.attempt, 5);
    }
}