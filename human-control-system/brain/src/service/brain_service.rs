//! Brain gRPC Service Implementation

use crate::brain_proto::brain_service_server::BrainService;
use crate::brain_proto::*;
use crate::bt::{BehaviorTree, NodeId, NodeStatus, NodeType, Blackboard};
use crate::bt::executor::{Executor, ExecutorConfig};
use crate::wasm::PluginExecutor;
use crate::wasm::runtime::WasmtimeRuntime;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};
use tracing::{debug, info, warn, error};
use uuid::Uuid;
use std::time::Instant;

/// Brain service state
pub struct BrainServiceState {
    pub trees: Arc<RwLock<HashMap<String, TreeState>>>,
    pub plugin_executor: Arc<PluginExecutor>,
    pub config: crate::config::BrainConfig,
    pub start_time: Instant,
}

/// State of a loaded behavior tree
pub struct TreeState {
    pub tree: BehaviorTree,
    pub executor: Executor,
    pub status: NodeStatus,
    pub created_at: Instant,
    pub last_tick: Option<Instant>,
    pub tick_count: u64,
}

/// Brain gRPC service
pub struct BrainServiceImpl {
    state: Arc<BrainServiceState>,
}

impl BrainServiceImpl {
    pub fn new(config: crate::config::BrainConfig) -> Self {
        let wasm_runtime = Arc::new(WasmtimeRuntime::new(config.wasm.clone().into()).unwrap());
        let plugin_executor = Arc::new(PluginExecutor::new(wasm_runtime));
        
        let state = BrainServiceState {
            trees: Arc::new(RwLock::new(HashMap::new())),
            plugin_executor,
            config,
            start_time: Instant::now(),
        };
        
        Self {
            state: Arc::new(state),
        }
    }
    
    pub fn state(&self) -> &Arc<BrainServiceState> {
        &self.state
    }
}

#[tonic::async_trait]
impl BrainService for BrainServiceImpl {
    async fn execute_tree(
        &self,
        request: Request<ExecuteTreeRequest>,
    ) -> Result<Response<ExecuteTreeResponse>, Status> {
        let req = request.into_inner();
        let tree_id = req.tree_id.clone();
        
        info!(tree_id = %tree_id, "ExecuteTree request");
        
        // Parse tree definition
        let tree = match BehaviorTree::from_json(&req.tree_definition) {
            Ok(t) => t,
            Err(e) => {
                return Ok(Response::new(ExecuteTreeResponse {
                    success: false,
                    tree_id,
                    error: format!("Failed to parse tree: {}", e),
                    final_status: TreeStatus::TreeStatusError as i32,
                    final_blackboard: HashMap::new(),
                    ticks_executed: 0,
                    execution_time_ms: 0.0,
                }));
            }
        };
        
        // Validate tree
        if let Err(e) = tree.validate() {
            return Ok(Response::new(ExecuteTreeResponse {
                success: false,
                tree_id,
                error: format!("Tree validation failed: {}", e),
                final_status: TreeStatus::TreeStatusError as i32,
                final_blackboard: HashMap::new(),
                ticks_executed: 0,
                execution_time_ms: 0.0,
            }));
        }
        
        // Set initial blackboard
        let mut blackboard = tree.blackboard.clone();
        for (k, v) in req.initial_blackboard {
            blackboard.set(k, serde_json::Value::String(v));
        }
        
        // Create executor
        let mut executor = Executor::new(tree)
            .with_config(ExecutorConfig {
                max_depth: self.state.config.behavior_tree.max_depth,
                max_ticks: None,
                tick_timeout_ms: self.state.config.server.tick_timeout_ms,
                enable_visualization: self.state.config.behavior_tree.enable_visualization,
            })
            .with_action_executor(Arc::new(crate::bt::action::ActionExecutor::new(crate::bt::action::ActionRegistry::new()).with_plugin_executor(self.state.plugin_executor.clone())))
            .with_condition_executor(Arc::new(crate::bt::condition::ConditionExecutor::new(crate::bt::condition::ConditionRegistry::new()).with_plugin_executor(self.state.plugin_executor.clone())))
            .with_plugin_executor(self.state.plugin_executor.clone());
        
        // Execute tree
        let start = Instant::now();
        let result = executor.execute().await;
        let elapsed = start.elapsed().as_millis() as f64;
        
        // Store if persistence requested
        if req.persist_state {
            let tree_state = TreeState {
                tree: executor.tree().clone(),
                executor,
                status: result.status,
                created_at: Instant::now(),
                last_tick: Some(Instant::now()),
                tick_count: result.tick_count,
            };
            self.state.trees.write().await.insert(tree_id.clone(), tree_state);
        }
        
        let final_bb: HashMap<String, String> = result.blackboard.snapshot()
            .into_iter()
            .map(|(k, v)| (k, v.to_string()))
            .collect();
        
        Ok(Response::new(ExecuteTreeResponse {
            success: matches!(result.status, NodeStatus::Success),
            tree_id,
            error: if matches!(result.status, NodeStatus::Error) { "Execution error".to_string() } else { String::new() },
            final_status: match result.status {
                NodeStatus::Success => TreeStatus::TreeStatusSuccess as i32,
                NodeStatus::Failure => TreeStatus::TreeStatusFailure as i32,
                NodeStatus::Running => TreeStatus::TreeStatusRunning as i32,
                NodeStatus::Error => TreeStatus::TreeStatusError as i32,
                NodeStatus::Stopped => TreeStatus::TreeStatusStopped as i32,
                _ => TreeStatus::TreeStatusError as i32,
            },
            final_blackboard: final_bb,
            ticks_executed: result.tick_count,
            execution_time_ms: elapsed,
        }))
    }
    
    async fn tick_tree(
        &self,
        request: Request<TickTreeRequest>,
    ) -> Result<Response<TickTreeResponse>, Status> {
        let req = request.into_inner();
        let tree_id = req.tree_id;
        
        let mut trees = self.state.trees.write().await;
        let tree_state = match trees.get_mut(&tree_id) {
            Some(s) => s,
            None => {
                return Ok(Response::new(TickTreeResponse {
                    success: false,
                    status: TreeStatus::TreeStatusNotLoaded as i32,
                    blackboard: HashMap::new(),
                    error: "Tree not found".to_string(),
                    tick_time_ms: 0.0,
                }));
            }
        };
        
        // Apply blackboard updates
        for (k, v) in req.blackboard_updates {
            tree_state.tree.blackboard.set(k, serde_json::Value::String(v));
        }
        
        // Execute single tick
        let start = Instant::now();
        let result = tree_state.executor.tick().await;
        let elapsed = start.elapsed().as_millis() as f64;
        
        tree_state.status = result.status;
        tree_state.last_tick = Some(Instant::now());
        tree_state.tick_count = result.tick_count;
        
        let bb: HashMap<String, String> = result.blackboard.snapshot()
            .into_iter()
            .map(|(k, v)| (k, v.to_string()))
            .collect();
        
        Ok(Response::new(TickTreeResponse {
            success: matches!(result.status, NodeStatus::Success | NodeStatus::Running),
            status: match result.status {
                NodeStatus::Success => TreeStatus::TreeStatusSuccess as i32,
                NodeStatus::Failure => TreeStatus::TreeStatusFailure as i32,
                NodeStatus::Running => TreeStatus::TreeStatusRunning as i32,
                NodeStatus::Error => TreeStatus::TreeStatusError as i32,
                NodeStatus::Stopped => TreeStatus::TreeStatusStopped as i32,
                _ => TreeStatus::TreeStatusError as i32,
            },
            blackboard: bb,
            error: if matches!(result.status, NodeStatus::Error) { "Tick error".to_string() } else { String::new() },
            tick_time_ms: elapsed,
        }))
    }
    
    async fn get_tree_status(
        &self,
        request: Request<GetTreeStatusRequest>,
    ) -> Result<Response<GetTreeStatusResponse>, Status> {
        let req = request.into_inner();
        let tree_id = req.tree_id.clone();
        
        let trees = self.state.trees.read().await;
        let tree_state = match trees.get(&tree_id) {
            Some(s) => s,
            None => {
                return Ok(Response::new(GetTreeStatusResponse {
                    success: false,
                    status: TreeStatus::TreeStatusNotLoaded as i32,
                    blackboard: HashMap::new(),
                    ticks_executed: 0,
                    error: "Tree not found".to_string(),
                }));
            }
        };
        
        let bb: HashMap<String, String> = tree_state.tree.blackboard.snapshot()
            .into_iter()
            .map(|(k, v)| (k, v.to_string()))
            .collect();
        
        Ok(Response::new(GetTreeStatusResponse {
            success: true,
            status: match tree_state.status {
                NodeStatus::Success => TreeStatus::TreeStatusSuccess as i32,
                NodeStatus::Failure => TreeStatus::TreeStatusFailure as i32,
                NodeStatus::Running => TreeStatus::TreeStatusRunning as i32,
                NodeStatus::Error => TreeStatus::TreeStatusError as i32,
                NodeStatus::Stopped => TreeStatus::TreeStatusStopped as i32,
                _ => TreeStatus::TreeStatusError as i32,
            },
            blackboard: bb,
            ticks_executed: tree_state.tick_count,
            error: String::new(),
        }))
    }
    
    async fn stop_tree(
        &self,
        request: Request<StopTreeRequest>,
    ) -> Result<Response<StopTreeResponse>, Status> {
        let req = request.into_inner();
        let tree_id = req.tree_id;
        
        let mut trees = self.state.trees.write().await;
        if let Some(tree_state) = trees.get_mut(&tree_id) {
            tree_state.executor.stop();
            if req.force {
                trees.remove(&tree_id);
            }
            Ok(Response::new(StopTreeResponse {
                success: true,
                error: String::new(),
            }))
        } else {
            Ok(Response::new(StopTreeResponse {
                success: false,
                error: "Tree not found".to_string(),
            }))
        }
    }
    
    async fn load_tree(
        &self,
        request: Request<LoadTreeRequest>,
    ) -> Result<Response<LoadTreeResponse>, Status> {
        let req = request.into_inner();
        let tree_id = req.tree_id.clone();
        
        let tree = match BehaviorTree::from_json(&req.tree_definition) {
            Ok(t) => t,
            Err(e) => {
                return Ok(Response::new(LoadTreeResponse {
                    success: false,
                    tree_id,
                    error: format!("Failed to parse tree: {}", e),
                }));
            }
        };
        
        if let Err(e) = tree.validate() {
            return Ok(Response::new(LoadTreeResponse {
                success: false,
                tree_id,
                error: format!("Tree validation failed: {}", e),
            }));
        }
        
        // Set initial blackboard
        let mut blackboard = tree.blackboard.clone();
        for (k, v) in req.initial_blackboard {
            blackboard.set(k, serde_json::Value::String(v));
        }
        
        let mut executor = Executor::new(tree)
            .with_config(ExecutorConfig {
                max_depth: self.state.config.behavior_tree.max_depth,
                max_ticks: None,
                tick_timeout_ms: self.state.config.server.tick_timeout_ms,
                enable_visualization: self.state.config.behavior_tree.enable_visualization,
            })
            .with_action_executor(Arc::new(crate::bt::action::ActionExecutor::new(crate::bt::action::ActionRegistry::new()).with_plugin_executor(self.state.plugin_executor.clone())))
            .with_condition_executor(Arc::new(crate::bt::condition::ConditionExecutor::new(crate::bt::condition::ConditionRegistry::new()).with_plugin_executor(self.state.plugin_executor.clone())))
            .with_plugin_executor(self.state.plugin_executor.clone());
        
        let tree_state = TreeState {
            tree: executor.tree().clone(),
            executor,
            status: NodeStatus::Pending,
            created_at: Instant::now(),
            last_tick: None,
            tick_count: 0,
        };
        
        self.state.trees.write().await.insert(tree_id.clone(), tree_state);
        
        Ok(Response::new(LoadTreeResponse {
            success: true,
            tree_id,
            error: String::new(),
        }))
    }
    
    async fn unload_tree(
        &self,
        request: Request<UnloadTreeRequest>,
    ) -> Result<Response<UnloadTreeResponse>, Status> {
        let req = request.into_inner();
        let tree_id = req.tree_id;
        
        let mut trees = self.state.trees.write().await;
        if trees.remove(&tree_id).is_some() {
            Ok(Response::new(UnloadTreeResponse {
                success: true,
                error: String::new(),
            }))
        } else {
            Ok(Response::new(UnloadTreeResponse {
                success: false,
                error: "Tree not found".to_string(),
            }))
        }
    }
    
    async fn list_trees(
        &self,
        _request: Request<ListTreesRequest>,
    ) -> Result<Response<ListTreesResponse>, Status> {
        let trees = self.state.trees.read().await;
        let tree_infos: Vec<TreeInfo> = trees.iter().map(|(id, state)| TreeInfo {
            tree_id: id.clone(),
            status: match state.status {
                NodeStatus::Success => TreeStatus::TreeStatusSuccess as i32,
                NodeStatus::Failure => TreeStatus::TreeStatusFailure as i32,
                NodeStatus::Running => TreeStatus::TreeStatusRunning as i32,
                NodeStatus::Error => TreeStatus::TreeStatusError as i32,
                NodeStatus::Stopped => TreeStatus::TreeStatusStopped as i32,
                _ => TreeStatus::TreeStatusError as i32,
            },
            ticks_executed: state.tick_count,
            root_node_type: format!("{:?}", state.tree.nodes.get(&state.tree.root).map(|n| n.node_type).unwrap_or(NodeType::Action)),
            node_count: state.tree.node_count() as u32,
        }).collect();
        
        Ok(Response::new(ListTreesResponse { trees: tree_infos }))
    }
    
    async fn get_tree_visualization(
        &self,
        request: Request<GetTreeVisualizationRequest>,
    ) -> Result<Response<GetTreeVisualizationResponse>, Status> {
        let req = request.into_inner();
        let tree_id = req.tree_id.clone();
        
        let trees = self.state.trees.read().await;
        let tree_state = match trees.get(&tree_id) {
            Some(s) => s,
            None => {
                return Ok(Response::new(GetTreeVisualizationResponse {
                    success: false,
                    visualization: String::new(),
                    error: "Tree not found".to_string(),
                }));
            }
        };
        
        let visualization = match req.format() {
            VisualizationFormat::VisualizationFormatDot => tree_state.executor.to_dot(),
            VisualizationFormat::VisualizationFormatJson => {
                serde_json::to_string_pretty(&tree_state.executor.to_json_visualization()).unwrap_or_default()
            }
            VisualizationFormat::VisualizationFormatMermaid => {
                // Generate Mermaid diagram
                format!("```mermaid\ngraph TD\n{}\n```", tree_state.executor.to_dot().replace("digraph BehaviorTree {", "").replace("}", ""))
            }
            _ => String::new(),
        };
        
        Ok(Response::new(GetTreeVisualizationResponse {
            success: true,
            visualization,
            error: String::new(),
        }))
    }
    
    async fn reload_plugins(
        &self,
        request: Request<ReloadPluginsRequest>,
    ) -> Result<Response<ReloadPluginsResponse>, Status> {
        let req = request.into_inner();
        let mut results = Vec::new();
        
        for path in req.plugin_paths {
            let name = std::path::Path::new(&path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&path);
            
            match self.state.plugin_executor.reload_plugin(name, std::path::Path::new(&path)).await {
                Ok(_) => results.push(PluginLoadResult {
                    plugin_path: path.clone(),
                    success: true,
                    plugin_name: name.to_string(),
                    error: String::new(),
                }),
                Err(e) => results.push(PluginLoadResult {
                    plugin_path: path.clone(),
                    success: false,
                    plugin_name: name.to_string(),
                    error: e.to_string(),
                }),
            }
        }
        
        let success = results.iter().all(|r| r.success);
        
        Ok(Response::new(ReloadPluginsResponse {
            success,
            results,
            error: if success { String::new() } else { "Some plugins failed to load".to_string() },
        }))
    }
    
    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let uptime = self.state.start_time.elapsed().as_secs();
        let mut components = HashMap::new();
        components.insert("wasm_runtime".to_string(), "healthy".to_string());
        components.insert("plugin_system".to_string(), "healthy".to_string());
        components.insert("behavior_tree".to_string(), "healthy".to_string());
        
        Ok(Response::new(HealthCheckResponse {
            healthy: true,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: uptime,
            components,
        }))
    }
    
    async fn get_metrics(
        &self,
        _request: Request<GetMetricsRequest>,
    ) -> Result<Response<GetMetricsResponse>, Status> {
        // Generate Prometheus metrics
        let metrics = format!(
            "# HELP brain_trees_loaded Total number of loaded trees\n# TYPE brain_trees_loaded gauge\nbrain_trees_loaded {}\n",
            self.state.trees.read().await.len()
        );
        
        Ok(Response::new(GetMetricsResponse {
            success: true,
            metrics,
            error: String::new(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_service_creation() {
        let config = crate::config::BrainConfig::default();
        let service = BrainServiceImpl::new(config);
        // Just verify it creates
    }
}