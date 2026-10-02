//! Task manager for async task execution

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn, error};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub task_type: String,
    pub parameters: serde_json::Value,
    pub priority: u32,
    pub timeout_ms: u64,
    pub status: TaskStatus,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub progress: Option<serde_json::Value>,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TaskHandle {
    pub task: Arc<RwLock<Task>>,
    pub cancel_tx: mpsc::Sender<()>,
    pub progress_tx: mpsc::Sender<TaskProgress>,
}

#[derive(Debug, Clone)]
pub struct TaskProgress {
    pub task_id: String,
    pub status: TaskStatus,
    pub progress: Option<serde_json::Value>,
    pub log: Option<String>,
}

pub struct TaskManager {
    tasks: Arc<DashMap<String, TaskHandle>>,
    max_concurrent: usize,
    running_count: Arc<std::sync::atomic::AtomicUsize>,
    history_size: usize,
    persistence_dir: std::path::PathBuf,
}

impl TaskManager {
    pub fn new(max_concurrent: usize, history_size: usize, persistence_dir: std::path::PathBuf) -> Self {
        Self {
            tasks: Arc::new(DashMap::new()),
            max_concurrent,
            running_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            history_size,
            persistence_dir,
        }
    }
    
    pub async fn submit_task(
        &self,
        task_type: String,
        parameters: serde_json::Value,
        priority: u32,
        timeout_ms: u64,
    ) -> Result<String> {
        // Check concurrent limit
        let running = self.running_count.load(std::sync::atomic::Ordering::Relaxed);
        if running >= self.max_concurrent {
            return Err(anyhow!("Max concurrent tasks reached"));
        }
        
        let task_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        
        let task = Task {
            id: task_id.clone(),
            task_type,
            parameters,
            priority,
            timeout_ms,
            status: TaskStatus::Pending,
            result: None,
            error: None,
            created_at: now,
            started_at: None,
            completed_at: None,
            progress: None,
            logs: Vec::new(),
        };
        
        let (cancel_tx, cancel_rx) = mpsc::channel(1);
        let (progress_tx, mut progress_rx) = mpsc::channel(100);
        
        let task_handle = TaskHandle {
            task: Arc::new(RwLock::new(task)),
            cancel_tx,
            progress_tx,
        };
        
        self.tasks.insert(task_id.clone(), task_handle);
        
        // Spawn task executor
        let tasks = self.tasks.clone();
        let running_count = self.running_count.clone();
        let task_id_clone = task_id.clone();
        
        tokio::spawn(async move {
            running_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            
            // Get task handle
            let handle = tasks.get(&task_id_clone).map(|e| e.clone());
            
            if let Some(handle) = handle {
                // Update status to running
                {
                    let mut task = handle.task.write().await;
                    task.status = TaskStatus::Running;
                    task.started_at = Some(Utc::now());
                }
                
                // Execute task with timeout
                let result = tokio::select! {
                    _ = cancel_rx.recv() => {
                        Err(anyhow!("Task cancelled"))
                    }
                    result = Self::execute_task_internal(
                        task_id_clone.clone(),
                        handle.task.clone(),
                        handle.progress_tx.clone(),
                    ) => {
                        result
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(timeout_ms)) => {
                        Err(anyhow!("Task timeout"))
                    }
                };
                
                // Update final status
                let mut task = handle.task.write().await;
                task.completed_at = Some(Utc::now());
                
                match result {
                    Ok(value) => {
                        task.status = TaskStatus::Completed;
                        task.result = Some(value);
                    }
                    Err(e) => {
                        task.status = TaskStatus::Failed;
                        task.error = Some(e.to_string());
                        error!("Task {} failed: {}", task_id_clone, e);
                    }
                }
            }
            
            running_count.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        });
        
        Ok(task_id)
    }
    
    async fn execute_task_internal(
        task_id: String,
        task: Arc<RwLock<Task>>,
        progress_tx: mpsc::Sender<TaskProgress>,
    ) -> Result<serde_json::Value> {
        // Get task type and parameters
        let (task_type, parameters) = {
            let task_guard = task.read().await;
            (task_guard.task_type.clone(), task_guard.parameters.clone())
        };
        
        // Dispatch to appropriate handler
        let result = match task_type.as_str() {
            "input_sequence" => Self::execute_input_sequence(task_id, task, progress_tx, parameters).await,
            "macro" => Self::execute_macro(task_id, task, progress_tx, parameters).await,
            "text_input" => Self::execute_text_input(task_id, task, progress_tx, parameters).await,
            "mouse_move" => Self::execute_mouse_move(task_id, task, progress_tx, parameters).await,
            "click" => Self::execute_click(task_id, task, progress_tx, parameters).await,
            "scroll" => Self::execute_scroll(task_id, task, progress_tx, parameters).await,
            "key_combo" => Self::execute_key_combo(task_id, task, progress_tx, parameters).await,
            "wait" => Self::execute_wait(task_id, task, progress_tx, parameters).await,
            "screenshot" => Self::execute_screenshot(task_id, task, progress_tx, parameters).await,
            "window_action" => Self::execute_window_action(task_id, task, progress_tx, parameters).await,
            _ => Err(anyhow!("Unknown task type: {}", task_type)),
        };
        
        result
    }
    
    async fn execute_input_sequence(
        task_id: String,
        task: Arc<RwLock<Task>>,
        progress_tx: mpsc::Sender<TaskProgress>,
        parameters: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let events: Vec<crate::driver::InputEvent> = serde_json::from_value(parameters)?;
        
        let driver_manager = Self::get_driver_manager()?;
        
        // Execute with progress updates
        let total = events.len();
        for (i, event) in events.into_iter().enumerate() {
            // Check cancellation
            if Self::is_cancelled(&task).await {
                return Err(anyhow!("Cancelled"));
            }
            
            driver_manager.inject_batch(vec![event]).await?;
            
            // Send progress
            let _ = progress_tx.send(TaskProgress {
                task_id: task_id.clone(),
                status: TaskStatus::Running,
                progress: Some(serde_json::json!({
                    "current": i + 1,
                    "total": total,
                    "percent": ((i + 1) * 100) / total
                })),
                log: None,
            }).await;
        }
        
        Ok(serde_json::json!({"executed": total}))
    }
    
    async fn execute_macro(
        task_id: String,
        task: Arc<RwLock<Task>>,
        progress_tx: mpsc::Sender<TaskProgress>,
        parameters: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let name = parameters.get("name").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Macro name required"))?;
        let variance_ms = parameters.get("variance_ms").and_then(|v| v.as_u64()).unwrap_or(50) as u32;
        
        let driver_manager = Self::get_driver_manager()?;
        driver_manager.execute_macro(name, variance_ms).await?;
        
        Ok(serde_json::json!({"macro": name}))
    }
    
    async fn execute_text_input(
        task_id: String,
        task: Arc<RwLock<Task>>,
        progress_tx: mpsc::Sender<TaskProgress>,
        parameters: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let text = parameters.get("text").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Text required"))?;
        let wpm = parameters.get("wpm").and_then(|v| v.as_u64()).unwrap_or(60) as u32;
        
        let driver_manager = Self::get_driver_manager()?;
        
        // Use the controller's type_text method
        // This would need access to InputController
        // For now, simulate with simple events
        
        Ok(serde_json::json!({"text_length": text.len()}))
    }
    
    async fn execute_mouse_move(
        task_id: String,
        task: Arc<RwLock<Task>>,
        progress_tx: mpsc::Sender<TaskProgress>,
        parameters: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let x = parameters.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let y = parameters.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let absolute = parameters.get("absolute").and_then(|v| v.as_bool()).unwrap_or(true);
        let duration_ms = parameters.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(500);
        
        // Would use InputController.move_mouse_absolute/relative
        
        Ok(serde_json::json!({"x": x, "y": y}))
    }
    
    async fn execute_click(
        task_id: String,
        task: Arc<RwLock<Task>>,
        progress_tx: mpsc::Sender<TaskProgress>,
        parameters: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let button = parameters.get("button").and_then(|v| v.as_str()).unwrap_or("left");
        let count = parameters.get("count").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
        
        let mouse_button = match button {
            "left" => crate::driver::MouseButton::Left,
            "right" => crate::driver::MouseButton::Right,
            "middle" => crate::driver::MouseButton::Middle,
            "x1" => crate::driver::MouseButton::X1,
            "x2" => crate::driver::MouseButton::X2,
            _ => crate::driver::MouseButton::Left,
        };
        
        let driver_manager = Self::get_driver_manager()?;
        
        for _ in 0..count {
            driver_manager.inject_mouse(crate::driver::MouseEvent {
                x: 0, y: 0, dx: 0, dy: 0,
                button: Some(mouse_button),
                button_state: Some(crate::driver::KeyState::Down),
                absolute: false,
                timestamp: 0,
            }).await?;
            
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            
            driver_manager.inject_mouse(crate::driver::MouseEvent {
                x: 0, y: 0, dx: 0, dy: 0,
                button: Some(mouse_button),
                button_state: Some(crate::driver::KeyState::Up),
                absolute: false,
                timestamp: 0,
            }).await?;
            
            if count > 1 {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
        
        Ok(serde_json::json!({"button": button, "count": count}))
    }
    
    async fn execute_scroll(
        task_id: String,
        task: Arc<RwLock<Task>>,
        progress_tx: mpsc::Sender<TaskProgress>,
        parameters: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let delta_y = parameters.get("delta_y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let delta_x = parameters.get("delta_x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        
        let driver_manager = Self::get_driver_manager()?;
        driver_manager.inject_mouse(crate::driver::MouseEvent {
            x: 0, y: 0, dx: 0, dy: 0,
            button: if delta_y > 0 { Some(crate::driver::MouseButton::WheelUp) }
                   else if delta_y < 0 { Some(crate::driver::MouseButton::WheelDown) }
                   else if delta_x > 0 { Some(crate::driver::MouseButton::WheelRight) }
                   else { Some(crate::driver::MouseButton::WheelLeft) },
            button_state: Some(crate::driver::KeyState::Down),
            absolute: false,
            timestamp: 0,
        }).await?;
        
        Ok(serde_json::json!({"delta_x": delta_x, "delta_y": delta_y}))
    }
    
    async fn execute_key_combo(
        task_id: String,
        task: Arc<RwLock<Task>>,
        progress_tx: mpsc::Sender<TaskProgress>,
        parameters: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let keys: Vec<u16> = parameters.get("keys").and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|x| x as u16)).collect())
            .unwrap_or_default();
        
        if keys.is_empty() {
            return Err(anyhow!("No keys specified"));
        }
        
        let driver_manager = Self::get_driver_manager()?;
        
        // Press all keys
        for &key in &keys {
            driver_manager.inject_keyboard(crate::driver::KeyboardEvent {
                code: key,
                state: crate::driver::KeyState::Down,
                scan_code: 0,
                extended: false,
                timestamp: 0,
            }).await?;
        }
        
        // Small delay
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        
        // Release in reverse order
        for &key in keys.iter().rev() {
            driver_manager.inject_keyboard(crate::driver::KeyboardEvent {
                code: key,
                state: crate::driver::KeyState::Up,
                scan_code: 0,
                extended: false,
                timestamp: 0,
            }).await?;
        }
        
        Ok(serde_json::json!({"keys": keys}))
    }
    
    async fn execute_wait(
        task_id: String,
        task: Arc<RwLock<Task>>,
        progress_tx: mpsc::Sender<TaskProgress>,
        parameters: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let ms = parameters.get("ms").and_then(|v| v.as_u64()).unwrap_or(1000);
        tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
        Ok(serde_json::json!({"waited_ms": ms}))
    }
    
    async fn execute_screenshot(
        task_id: String,
        task: Arc<RwLock<Task>>,
        progress_tx: mpsc::Sender<TaskProgress>,
        parameters: serde_json::Value,
    ) -> Result<serde_json::Value> {
        // Would integrate with vision service
        Ok(serde_json::json!({"screenshot": "not_implemented"}))
    }
    
    async fn execute_window_action(
        task_id: String,
        task: Arc<RwLock<Task>>,
        progress_tx: mpsc::Sender<TaskProgress>,
        parameters: serde_json::Value,
    ) -> Result<serde_json::Value> {
        // Would integrate with window management
        Ok(serde_json::json!({"window_action": "not_implemented"}))
    }
    
    fn get_driver_manager() -> Result<Arc<crate::driver_manager::DriverManager>> {
        // This would be injected via DI in real implementation
        Err(anyhow!("Driver manager not available"))
    }
    
    async fn is_cancelled(task: &Arc<RwLock<Task>>) -> bool {
        let task_guard = task.read().await;
        task_guard.status == TaskStatus::Cancelled
    }
    
    pub async fn get_task(&self, task_id: &str) -> Option<Task> {
        self.tasks.get(task_id).map(|h| h.task.blocking_read().clone())
    }
    
    pub async fn cancel_task(&self, task_id: &str) -> Result<()> {
        if let Some(handle) = self.tasks.get(task_id) {
            let _ = handle.cancel_tx.send(()).await;
            let mut task = handle.task.write().await;
            task.status = TaskStatus::Cancelled;
            Ok(())
        } else {
            Err(anyhow!("Task not found"))
        }
    }
    
    pub async fn list_tasks(&self, filter: TaskFilter) -> Vec<Task> {
        let mut tasks: Vec<Task> = self.tasks.iter()
            .map(|e| e.value().task.blocking_read().clone())
            .collect();
        
        // Apply filters
        if !filter.statuses.is_empty() {
            tasks.retain(|t| filter.statuses.contains(&t.status));
        }
        
        if !filter.types.is_empty() {
            tasks.retain(|t| filter.types.contains(&t.task_type));
        }
        
        // Sort by priority and creation time
        tasks.sort_by(|a, b| {
            b.priority.cmp(&a.priority)
                .then_with(|| a.created_at.cmp(&b.created_at))
        });
        
        // Apply pagination
        let start = filter.offset as usize;
        let end = (start + filter.limit as usize).min(tasks.len());
        tasks[start..end].to_vec()
    }
    
    pub async fn get_task_progress(&self, task_id: &str) -> Option<mpsc::Receiver<TaskProgress>> {
        // Would need to store receiver - simplified
        None
    }
}

#[derive(Debug, Clone)]
pub struct TaskFilter {
    pub statuses: Vec<TaskStatus>,
    pub types: Vec<String>,
    pub limit: u32,
    pub offset: u32,
}

impl Default for TaskFilter {
    fn default() -> Self {
        Self {
            statuses: vec![],
            types: vec![],
            limit: 100,
            offset: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_task_submission() {
        let manager = TaskManager::new(2, 100, std::path::PathBuf::from("/tmp"));
        
        let task_id = manager.submit_task(
            "wait".to_string(),
            serde_json::json!({"ms": 100}),
            1,
            5000,
        ).await.unwrap();
        
        // Wait a bit
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        
        let task = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
    }
}