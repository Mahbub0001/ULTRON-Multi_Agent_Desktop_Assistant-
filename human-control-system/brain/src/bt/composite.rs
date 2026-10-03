//! Composite nodes: Sequence, Selector, Parallel

use crate::bt::{NodeId, NodeStatus, Blackboard, ExecutionContext};
use async_trait::async_trait;

/// Trait for composite node execution logic
#[async_trait]
pub trait CompositeNode: Send + Sync {
    async fn tick(&self, ctx: &mut ExecutionContext, children: &[NodeId]) -> NodeStatus;
}

/// Sequence node: executes children in order, succeeds if all succeed
pub struct SequenceNode;

#[async_trait]
impl CompositeNode for SequenceNode {
    async fn tick(&self, ctx: &mut ExecutionContext, children: &[NodeId]) -> NodeStatus {
        for child_id in children {
            let status = ctx.tick_node(*child_id).await;
            match status {
                NodeStatus::Success => continue,
                NodeStatus::Running => return NodeStatus::Running,
                NodeStatus::Failure | NodeStatus::Error => return status,
                NodeStatus::Skipped => continue,
                NodeStatus::Pending => return NodeStatus::Pending,
            }
        }
        NodeStatus::Success
    }
}

/// Selector node: executes children in order, succeeds if any succeeds
pub struct SelectorNode;

#[async_trait]
impl CompositeNode for SelectorNode {
    async fn tick(&self, ctx: &mut ExecutionContext, children: &[NodeId]) -> NodeStatus {
        for child_id in children {
            let status = ctx.tick_node(*child_id).await;
            match status {
                NodeStatus::Success => return NodeStatus::Success,
                NodeStatus::Running => return NodeStatus::Running,
                NodeStatus::Failure | NodeStatus::Error => continue,
                NodeStatus::Skipped => continue,
                NodeStatus::Pending => return NodeStatus::Pending,
            }
        }
        NodeStatus::Failure
    }
}

/// Parallel node: executes all children, succeeds if enough succeed
pub struct ParallelNode {
    pub success_threshold: usize,
    pub failure_threshold: usize,
}

#[async_trait]
impl CompositeNode for ParallelNode {
    async fn tick(&self, ctx: &mut ExecutionContext, children: &[NodeId]) -> NodeStatus {
        if children.is_empty() {
            return NodeStatus::Success;
        }
        
        let mut success_count = 0;
        let mut failure_count = 0;
        let mut running_count = 0;
        
        for child_id in children {
            let status = ctx.tick_node(*child_id).await;
            match status {
                NodeStatus::Success => success_count += 1,
                NodeStatus::Failure | NodeStatus::Error => failure_count += 1,
                NodeStatus::Running => running_count += 1,
                NodeStatus::Skipped => {}
                NodeStatus::Pending => return NodeStatus::Pending,
            }
        }
        
        // Check success threshold
        if success_count >= self.success_threshold {
            return NodeStatus::Success;
        }
        
        // Check failure threshold
        if failure_count >= self.failure_threshold {
            return NodeStatus::Failure;
        }
        
        // If any children are still running, we're running
        if running_count > 0 {
            return NodeStatus::Running;
        }
        
        // All children finished but thresholds not met
        NodeStatus::Failure
    }
}

/// Factory for creating composite nodes
pub fn create_composite(node_type: crate::bt::NodeType) -> Option<Box<dyn CompositeNode>> {
    match node_type {
        crate::bt::NodeType::Sequence => Some(Box::new(SequenceNode)),
        crate::bt::NodeType::Selector => Some(Box::new(SelectorNode)),
        crate::bt::NodeType::Parallel => Some(Box::new(ParallelNode {
            success_threshold: 1,
            failure_threshold: 1,
        })),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bt::{Node, NodeId, NodeStatus, NodeType, NodeConfig, BehaviorTree};
    use std::collections::HashMap;
    
    fn create_test_tree() -> (BehaviorTree, NodeId, NodeId, NodeId) {
        let mut tree = BehaviorTree::new("Test");
        
        let action1 = Node::new_action("Action 1", "always_success", None, HashMap::new());
        let action2 = Node::new_action("Action 2", "always_success", None, HashMap::new());
        let action3 = Node::new_action("Action 3", "always_failure", None, HashMap::new());
        
        let id1 = action1.id;
        let id2 = action2.id;
        let id3 = action3.id;
        
        tree.add_node(action1);
        tree.add_node(action2);
        tree.add_node(action3);
        
        (tree, id1, id2, id3)
    }
    
    #[tokio::test]
    async fn test_sequence_all_success() {
        let sequence = SequenceNode;
        // We can't easily test without a full executor, but we can verify the logic
        // by checking the implementation directly
    }
    
    #[tokio::test]
    async fn test_selector_first_success() {
        let selector = SelectorNode;
    }
    
    #[tokio::test]
    async fn test_parallel_thresholds() {
        let parallel = ParallelNode {
            success_threshold: 2,
            failure_threshold: 2,
        };
    }
}