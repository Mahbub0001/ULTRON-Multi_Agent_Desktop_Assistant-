use crate::dsl::types::*;
use crate::client::grpc_client::GrpcClient;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

pub struct Interpreter {
    client: Arc<GrpcClient>,
    context: ExecutionContext,
    config: InterpreterConfig,
}

#[derive(Debug, Clone)]
pub struct InterpreterConfig {
    pub max_execution_time_ms: u64,
    pub max_loop_iterations: u32,
    pub max_recursion_depth: u32,
    pub default_step_timeout_ms: u32,
}

impl Default for InterpreterConfig {
    fn default() -> Self {
        Self {
            max_execution_time_ms: 300000,
            max_loop_iterations: 10000,
            max_recursion_depth: 100,
            default_step_timeout_ms: 10000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub success: bool,
    pub task_id: String,
    pub error: Option<String>,
    pub result: Option<serde_json::Value>,
    pub execution_time_ms: u64,
    pub step_results: Vec<StepResult>,
}

impl Interpreter {
    pub fn new(client: Arc<GrpcClient>, config: InterpreterConfig) -> Self {
        let task_id = uuid::Uuid::new_v4().to_string();
        Self {
            client,
            context: ExecutionContext::new(task_id),
            config,
        }
    }

    pub fn with_context(mut self, context: ExecutionContext) -> Self {
        self.context = context;
        self
    }

    pub fn with_variables(mut self, variables: HashMap<String, serde_json::Value>) -> Self {
        self.context.variables = variables;
        self
    }

    pub async fn execute(&mut self, task: TaskDefinition) -> Result<ExecutionResult> {
        let start_time = Instant::now();
        let task_id = self.context.task_id.clone();

        info!("Starting task execution: {} ({})", task.name, task_id);

        // Initialize variables with defaults
        for (name, def) in &task.variables {
            if let Some(default) = &def.default {
                self.context.set_var(name.clone(), default.clone());
            } else if def.required {
                return Ok(ExecutionResult {
                    success: false,
                    task_id,
                    error: Some(format!("Required variable '{}' not provided", name)),
                    result: None,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    step_results: vec![],
                });
            }
        }

        // Register macros
        for (name, macro_def) in &task.macros {
            self.context.set_var(format!("macro.{}", name), serde_json::to_value(macro_def)?);
        }

        let mut step_results = Vec::new();
        let mut success = true;
        let mut error = None;

        for step in &task.steps {
            if start_time.elapsed().as_millis() as u64 > self.config.max_execution_time_ms {
                error = Some("Task execution timeout".to_string());
                success = false;
                break;
            }

            let result = self.execute_step(step, 0).await;
            step_results.push(result.clone());

            if result.status == StepStatus::Failed {
                success = false;
                error = result.error.clone();
                break;
            }
        }

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        info!("Task execution completed: {} in {}ms", task_id, execution_time_ms);

        Ok(ExecutionResult {
            success,
            task_id,
            error,
            result: Some(serde_json::to_value(&self.context.variables)?),
            execution_time_ms,
            step_results,
        })
    }

    async fn execute_step(&mut self, step: &Step, depth: u32) -> StepResult {
        if depth > self.config.max_recursion_depth {
            return StepResult {
                step_id: self.get_step_id(step).to_string(),
                status: StepStatus::Failed,
                output: None,
                error: Some("Max recursion depth exceeded".to_string()),
                start_time: chrono::Utc::now(),
                end_time: Some(chrono::Utc::now()),
                sub_steps: vec![],
            };
        }

        let step_id = self.get_step_id(step).to_string();
        let start_time = chrono::Utc::now();

        // Check condition
        if let Some(condition) = self.get_condition(step) {
            if !self.evaluate_condition(&condition).await.unwrap_or(false) {
                return StepResult {
                    step_id,
                    status: StepStatus::Skipped,
                    output: None,
                    error: None,
                    start_time,
                    end_time: Some(chrono::Utc::now()),
                    sub_steps: vec![],
                };
            }
        }

        let result = match step {
            Step::Action(s) => self.execute_action(s).await,
            Step::Macro(s) => self.execute_macro(s).await,
            Step::Sequential(s) => self.execute_sequential(s, depth + 1).await,
            Step::Parallel(s) => self.execute_parallel(s, depth + 1).await,
            Step::Conditional(s) => self.execute_conditional(s, depth + 1).await,
            Step::Loop(s) => self.execute_loop(s, depth + 1).await,
            Step::Vision(s) => self.execute_vision(s).await,
            Step::Brain(s) => self.execute_brain(s).await,
            Step::Adapter(s) => self.execute_adapter(s).await,
            Step::Variable(s) => self.execute_variable(s).await,
            Step::Wait(s) => self.execute_wait(s).await,
            Step::Log(s) => self.execute_log(s).await,
        };

        let end_time = chrono::Utc::now();
        let mut final_result = result;
        final_result.step_id = step_id;
        final_result.start_time = start_time;
        final_result.end_time = Some(end_time);

        self.context.step_results.insert(final_result.step_id.clone(), final_result.clone());
        final_result
    }

    fn get_step_id(&self, step: &Step) -> &str {
        match step {
            Step::Action(s) => &s.id,
            Step::Macro(s) => &s.id,
            Step::Sequential(s) => &s.id,
            Step::Parallel(s) => &s.id,
            Step::Conditional(s) => &s.id,
            Step::Loop(s) => &s.id,
            Step::Vision(s) => &s.id,
            Step::Brain(s) => &s.id,
            Step::Adapter(s) => &s.id,
            Step::Variable(s) => &s.id,
            Step::Wait(s) => &s.id,
            Step::Log(s) => &s.id,
        }
    }

    fn get_condition(&self, step: &Step) -> Option<&String> {
        match step {
            Step::Action(s) => s.condition.as_ref(),
            Step::Macro(s) => s.condition.as_ref(),
            Step::Sequential(s) => s.condition.as_ref(),
            Step::Parallel(s) => s.condition.as_ref(),
            Step::Conditional(_) => None, // Handled separately
            Step::Loop(_) => None, // Handled separately
            Step::Vision(s) => s.condition.as_ref(),
            Step::Brain(s) => s.condition.as_ref(),
            Step::Adapter(s) => s.condition.as_ref(),
            Step::Variable(s) => s.condition.as_ref(),
            Step::Wait(_) => None, // Handled separately
            Step::Log(s) => s.condition.as_ref(),
        }
    }

    async fn evaluate_condition(&self, condition: &str) -> Result<bool> {
        // Resolve template variables
        let resolved = self.context.resolve_template(condition);
        
        // Use Starlark for evaluation
        let mut eval = starlark::environment::GlobalsBuilder::standard().build();
        for (key, value) in &self.context.variables {
            let starlark_val = json_to_starlark(value)?;
            eval.set(key, starlark_val);
        }

        let result = eval.eval(&resolved)?;
        Ok(result.to_bool())
    }

    async fn execute_action(&self, step: &ActionStep) -> StepResult {
        debug!("Executing action: {}", step.name);

        let timeout = step.timeout_ms.unwrap_or(self.config.default_step_timeout_ms);
        
        match tokio::time::timeout(
            Duration::from_millis(timeout as u64),
            self.execute_action_inner(step),
        ).await {
            Ok(Ok(output)) => StepResult {
                step_id: step.id.clone(),
                status: StepStatus::Completed,
                output: Some(output),
                error: None,
                start_time: chrono::Utc::now(),
                end_time: None,
                sub_steps: vec![],
            },
            Ok(Err(e)) => StepResult {
                step_id: step.id.clone(),
                status: StepStatus::Failed,
                output: None,
                error: Some(e.to_string()),
                start_time: chrono::Utc::now(),
                end_time: None,
                sub_steps: vec![],
            },
            Err(_) => StepResult {
                step_id: step.id.clone(),
                status: StepStatus::Failed,
                output: None,
                error: Some("Action timeout".to_string()),
                start_time: chrono::Utc::now(),
                end_time: None,
                sub_steps: vec![],
            },
        }
    }

    async fn execute_action_inner(&self, step: &ActionStep) -> Result<serde_json::Value> {
        match &step.action {
            ActionType::KeyDown { code, scan_code, extended } => {
                let event = hcs_agent_proto::KeyEvent {
                    code: *code,
                    state: hcs_agent_proto::KeyState::KeyStateDown as i32,
                    scan_code: scan_code.unwrap_or(0),
                    extended: extended.unwrap_or(false),
                    timestamp_us: 0,
                };
                self.client.inject_key(event).await?;
                Ok(serde_json::json!({"action": "key_down", "code": code}))
            }
            ActionType::KeyUp { code, scan_code, extended } => {
                let event = hcs_agent_proto::KeyEvent {
                    code: *code,
                    state: hcs_agent_proto::KeyState::KeyStateUp as i32,
                    scan_code: scan_code.unwrap_or(0),
                    extended: extended.unwrap_or(false),
                    timestamp_us: 0,
                };
                self.client.inject_key(event).await?;
                Ok(serde_json::json!({"action": "key_up", "code": code}))
            }
            ActionType::KeyPress { code, scan_code, extended, delay_ms } => {
                let down_event = hcs_agent_proto::KeyEvent {
                    code: *code,
                    state: hcs_agent_proto::KeyState::KeyStateDown as i32,
                    scan_code: scan_code.unwrap_or(0),
                    extended: extended.unwrap_or(false),
                    timestamp_us: 0,
                };
                self.client.inject_key(down_event).await?;

                if let Some(delay) = delay_ms {
                    sleep(Duration::from_millis(delay as u64)).await;
                } else {
                    sleep(Duration::from_millis(50)).await;
                }

                let up_event = hcs_agent_proto::KeyEvent {
                    code: *code,
                    state: hcs_agent_proto::KeyState::KeyStateUp as i32,
                    scan_code: scan_code.unwrap_or(0),
                    extended: extended.unwrap_or(false),
                    timestamp_us: 0,
                };
                self.client.inject_key(up_event).await?;

                Ok(serde_json::json!({"action": "key_press", "code": code}))
            }
            ActionType::MouseMove { x, y, absolute } => {
                let event = hcs_agent_proto::MouseEvent {
                    x: *x,
                    y: *y,
                    dx: 0,
                    dy: 0,
                    button: hcs_agent_proto::MouseButton::MouseButtonUnspecified as i32,
                    button_state: hcs_agent_proto::KeyState::KeyStateUnspecified as i32,
                    absolute: absolute.unwrap_or(true),
                    timestamp_us: 0,
                };
                self.client.inject_mouse(event).await?;
                Ok(serde_json::json!({"action": "mouse_move", "x": x, "y": y}))
            }
            ActionType::MouseClick { button, x, y, count } => {
                let btn = match button {
                    MouseButton::Left => hcs_agent_proto::MouseButton::MouseButtonLeft,
                    MouseButton::Right => hcs_agent_proto::MouseButton::MouseButtonRight,
                    MouseButton::Middle => hcs_agent_proto::MouseButton::MouseButtonMiddle,
                    MouseButton::X1 => hcs_agent_proto::MouseButton::MouseButtonX1,
                    MouseButton::X2 => hcs_agent_proto::MouseButton::MouseButtonX2,
                    MouseButton::WheelUp => hcs_agent_proto::MouseButton::MouseButtonWheelUp,
                    MouseButton::WheelDown => hcs_agent_proto::MouseButton::MouseButtonWheelDown,
                    MouseButton::WheelLeft => hcs_agent_proto::MouseButton::MouseButtonWheelLeft,
                    MouseButton::WheelRight => hcs_agent_proto::MouseButton::MouseButtonWheelRight,
                };

                if let Some(x) = x {
                    if let Some(y) = y {
                        let move_event = hcs_agent_proto::MouseEvent {
                            x: *x,
                            y: *y,
                            dx: 0,
                            dy: 0,
                            button: hcs_agent_proto::MouseButton::MouseButtonUnspecified as i32,
                            button_state: hcs_agent_proto::KeyState::KeyStateUnspecified as i32,
                            absolute: true,
                            timestamp_us: 0,
                        };
                        self.client.inject_mouse(move_event).await?;
                    }
                }

                for _ in 0..count.unwrap_or(1) {
                    let down_event = hcs_agent_proto::MouseEvent {
                        x: x.unwrap_or(0),
                        y: y.unwrap_or(0),
                        dx: 0,
                        dy: 0,
                        button: btn as i32,
                        button_state: hcs_agent_proto::KeyState::KeyStateDown as i32,
                        absolute: true,
                        timestamp_us: 0,
                    };
                    self.client.inject_mouse(down_event).await?;

                    sleep(Duration::from_millis(10)).await;

                    let up_event = hcs_agent_proto::MouseEvent {
                        x: x.unwrap_or(0),
                        y: y.unwrap_or(0),
                        dx: 0,
                        dy: 0,
                        button: btn as i32,
                        button_state: hcs_agent_proto::KeyState::KeyStateUp as i32,
                        absolute: true,
                        timestamp_us: 0,
                    };
                    self.client.inject_mouse(up_event).await?;

                    sleep(Duration::from_millis(50)).await;
                }

                Ok(serde_json::json!({"action": "mouse_click", "button": button}))
            }
            ActionType::MouseDown { button, x, y } => {
                let btn = self.mouse_button_to_proto(button);
                let event = hcs_agent_proto::MouseEvent {
                    x: x.unwrap_or(0),
                    y: y.unwrap_or(0),
                    dx: 0,
                    dy: 0,
                    button: btn as i32,
                    button_state: hcs_agent_proto::KeyState::KeyStateDown as i32,
                    absolute: true,
                    timestamp_us: 0,
                };
                self.client.inject_mouse(event).await?;
                Ok(serde_json::json!({"action": "mouse_down", "button": button}))
            }
            ActionType::MouseUp { button, x, y } => {
                let btn = self.mouse_button_to_proto(button);
                let event = hcs_agent_proto::MouseEvent {
                    x: x.unwrap_or(0),
                    y: y.unwrap_or(0),
                    dx: 0,
                    dy: 0,
                    button: btn as i32,
                    button_state: hcs_agent_proto::KeyState::KeyStateUp as i32,
                    absolute: true,
                    timestamp_us: 0,
                };
                self.client.inject_mouse(event).await?;
                Ok(serde_json::json!({"action": "mouse_up", "button": button}))
            }
            ActionType::MouseScroll { dx, dy } => {
                let event = hcs_agent_proto::MouseEvent {
                    x: 0,
                    y: 0,
                    dx: *dx,
                    dy: *dy,
                    button: hcs_agent_proto::MouseButton::MouseButtonUnspecified as i32,
                    button_state: hcs_agent_proto::KeyState::KeyStateUnspecified as i32,
                    absolute: false,
                    timestamp_us: 0,
                };
                self.client.inject_mouse(event).await?;
                Ok(serde_json::json!({"action": "mouse_scroll", "dx": dx, "dy": dy}))
            }
            ActionType::Delay { microseconds } => {
                sleep(Duration::from_micros(*microseconds)).await;
                Ok(serde_json::json!({"action": "delay", "microseconds": microseconds}))
            }
            ActionType::TypeText { text, delay_ms } => {
                let delay = delay_ms.unwrap_or(20);
                for ch in text.chars() {
                    // Simplified - would need proper key mapping
                    let code = self.char_to_key_code(ch);
                    if code > 0 {
                        let down_event = hcs_agent_proto::KeyEvent {
                            code,
                            state: hcs_agent_proto::KeyState::KeyStateDown as i32,
                            scan_code: 0,
                            extended: false,
                            timestamp_us: 0,
                        };
                        self.client.inject_key(down_event).await?;
                        
                        sleep(Duration::from_millis(delay as u64)).await;
                        
                        let up_event = hcs_agent_proto::KeyEvent {
                            code,
                            state: hcs_agent_proto::KeyState::KeyStateUp as i32,
                            scan_code: 0,
                            extended: false,
                            timestamp_us: 0,
                        };
                        self.client.inject_key(up_event).await?;
                    }
                }
                Ok(serde_json::json!({"action": "type_text", "text": text}))
            }
        }
    }

    fn mouse_button_to_proto(&self, button: &MouseButton) -> hcs_agent_proto::MouseButton {
        match button {
            MouseButton::Left => hcs_agent_proto::MouseButton::MouseButtonLeft,
            MouseButton::Right => hcs_agent_proto::MouseButton::MouseButtonRight,
            MouseButton::Middle => hcs_agent_proto::MouseButton::MouseButtonMiddle,
            MouseButton::X1 => hcs_agent_proto::MouseButton::MouseButtonX1,
            MouseButton::X2 => hcs_agent_proto::MouseButton::MouseButtonX2,
            MouseButton::WheelUp => hcs_agent_proto::MouseButton::MouseButtonWheelUp,
            MouseButton::WheelDown => hcs_agent_proto::MouseButton::MouseButtonWheelDown,
            MouseButton::WheelLeft => hcs_agent_proto::MouseButton::MouseButtonWheelLeft,
            MouseButton::WheelRight => hcs_agent_proto::MouseButton::MouseButtonWheelRight,
        }
    }

    fn char_to_key_code(&self, ch: char) -> u32 {
        // Simplified mapping - would need proper keyboard layout handling
        match ch {
            'a'..='z' => ch as u32 - 'a' as u32 + 0x41,
            'A'..='Z' => ch as u32 - 'A' as u32 + 0x41,
            '0'..='9' => ch as u32 - '0' as u32 + 0x30,
            ' ' => 0x20,
            '\n' => 0x0D,
            '\t' => 0x09,
            _ => 0,
        }
    }

    async fn execute_macro(&self, step: &MacroStep) -> StepResult {
        debug!("Executing macro: {}", step.macro_name);

        // Look up macro definition
        let macro_key = format!("macro.{}", step.macro_name);
        let macro_value = self.context.get_var(&macro_key);
        
        if macro_value.is_none() {
            return StepResult {
                step_id: step.id.clone(),
                status: StepStatus::Failed,
                output: None,
                error: Some(format!("Macro '{}' not found", step.macro_name)),
                start_time: chrono::Utc::now(),
                end_time: None,
                sub_steps: vec![],
            };
        }

        // For now, execute macro via gRPC
        let params = step.parameters.clone();
        match self.client.execute_macro(&step.macro_name, params).await {
            Ok(result) => StepResult {
                step_id: step.id.clone(),
                status: StepStatus::Completed,
                output: Some(result),
                error: None,
                start_time: chrono::Utc::now(),
                end_time: None,
                sub_steps: vec![],
            },
            Err(e) => StepResult {
                step_id: step.id.clone(),
                status: StepStatus::Failed,
                output: None,
                error: Some(e.to_string()),
                start_time: chrono::Utc::now(),
                end_time: None,
                sub_steps: vec![],
            },
        }
    }

    async fn execute_sequential(&mut self, step: &SequentialStep, depth: u32) -> StepResult {
        debug!("Executing sequential block: {}", step.name);
        let mut sub_results = Vec::new();
        let mut overall_success = true;

        for nested_step in &step.steps {
            let result = self.execute_step(nested_step, depth).await;
            sub_results.push(result.clone());

            if result.status == StepStatus::Failed {
                overall_success = false;
                break;
            }
        }

        StepResult {
            step_id: step.id.clone(),
            status: if overall_success { StepStatus::Completed } else { StepStatus::Failed },
            output: Some(serde_json::json!({"steps": sub_results.len()})),
            error: if overall_success { None } else { Some("Sequential step failed".to_string()) },
            start_time: chrono::Utc::now(),
            end_time: None,
            sub_steps: sub_results,
        }
    }

    async fn execute_parallel(&mut self, step: &ParallelStep, depth: u32) -> StepResult {
        debug!("Executing parallel block: {}", step.name);
        
        let futures: Vec<_> = step.steps.iter().map(|s| self.execute_step(s, depth)).collect();
        let results = futures::future::join_all(futures).await;

        let mut all_success = true;
        for result in &results {
            if result.status == StepStatus::Failed {
                all_success = false;
                if step.fail_fast {
                    break;
                }
            }
        }

        StepResult {
            step_id: step.id.clone(),
            status: if all_success { StepStatus::Completed } else { StepStatus::Failed },
            output: Some(serde_json::json!({"steps": results.len()})),
            error: if all_success { None } else { Some("Parallel step failed".to_string()) },
            start_time: chrono::Utc::now(),
            end_time: None,
            sub_steps: results,
        }
    }

    async fn execute_conditional(&mut self, step: &ConditionalStep, depth: u32) -> StepResult {
        debug!("Executing conditional: {}", step.name);

        let condition_result = self.evaluate_condition(&step.condition).await.unwrap_or(false);
        
        let branch = if condition_result {
            &step.then_branch
        } else if step.else_branch.is_some() {
            &step.else_branch.as_ref().unwrap()
        } else {
            return StepResult {
                step_id: step.id.clone(),
                status: StepStatus::Completed,
                output: Some(serde_json::json!({"condition": false, "branch": "none"})),
                error: None,
                start_time: chrono::Utc::now(),
                end_time: None,
                sub_steps: vec![],
            };
        };

        let result = self.execute_step(branch, depth).await;
        StepResult {
            step_id: step.id.clone(),
            status: result.status,
            output: Some(serde_json::json!({
                "condition": condition_result,
                "branch": if condition_result { "then" } else { "else" },
                "result": result.output
            })),
            error: result.error,
            start_time: chrono::Utc::now(),
            end_time: None,
            sub_steps: vec![result],
        }
    }

    async fn execute_loop(&mut self, step: &LoopStep, depth: u32) -> StepResult {
        debug!("Executing loop: {}", step.name);
        let mut sub_results = Vec::new();
        let mut iterations = 0;

        match step.loop_type {
            LoopType::Count => {
                let count = step.count.unwrap_or(0) as usize;
                for i in 0..count {
                    if iterations >= self.config.max_loop_iterations as usize {
                        warn!("Loop {} exceeded max iterations", step.id);
                        break;
                    }
                    if let Some(iter_var) = &step.iterator {
                        self.context.set_var(iter_var.clone(), serde_json::json!(i));
                    }
                    self.context.loop_counters.insert(step.id.clone(), i as u32);

                    let result = self.execute_step(&step.body, depth).await;
                    sub_results.push(result.clone());

                    if result.status == StepStatus::Failed {
                        break;
                    }
                    iterations += 1;
                }
            }
            LoopType::While => {
                let condition = step.condition.as_ref().unwrap_or(&"true".to_string()).clone();
                while iterations < self.config.max_loop_iterations as usize {
                    let condition_result = self.evaluate_condition(&condition).await.unwrap_or(false);
                    if !condition_result {
                        break;
                    }
                    if let Some(iter_var) = &step.iterator {
                        self.context.set_var(iter_var.clone(), serde_json::json!(iterations));
                    }
                    self.context.loop_counters.insert(step.id.clone(), iterations as u32);

                    let result = self.execute_step(&step.body, depth).await;
                    sub_results.push(result.clone());

                    if result.status == StepStatus::Failed {
                        break;
                    }
                    iterations += 1;
                }
            }
            LoopType::ForEach => {
                let collection_expr = step.collection.as_ref().unwrap_or(&"[]".to_string()).clone();
                let collection_value = self.evaluate_expression(&collection_expr).await;
                let collection = match collection_value {
                    serde_json::Value::Array(arr) => arr,
                    _ => vec![],
                };

                for (i, item) in collection.into_iter().enumerate() {
                    if iterations >= self.config.max_loop_iterations as usize {
                        break;
                    }
                    if let Some(iter_var) = &step.iterator {
                        self.context.set_var(iter_var.clone(), item);
                    }
                    self.context.loop_counters.insert(step.id.clone(), i as u32);

                    let result = self.execute_step(&step.body, depth).await;
                    sub_results.push(result.clone());

                    if result.status == StepStatus::Failed {
                        break;
                    }
                    iterations += 1;
                }
            }
        }

        StepResult {
            step_id: step.id.clone(),
            status: if sub_results.iter().all(|r| r.status != StepStatus::Failed) { 
                StepStatus::Completed 
            } else { 
                StepStatus::Failed 
            },
            output: Some(serde_json::json!({"iterations": iterations})),
            error: None,
            start_time: chrono::Utc::now(),
            end_time: None,
            sub_steps: sub_results,
        }
    }

    async fn evaluate_expression(&self, expr: &str) -> serde_json::Value {
        let resolved = self.context.resolve_template(expr);
        // Simple evaluation - in practice would use Starlark
        if resolved.starts_with('[') && resolved.ends_with(']') {
            serde_json::from_str(&resolved).unwrap_or(serde_json::Value::Array(vec![]))
        } else {
            serde_json::Value::Null
        }
    }

    async fn execute_vision(&self, step: &VisionStep) -> StepResult {
        debug!("Executing vision step: {}", step.name);

        match &step.operation {
            VisionOperation::CaptureScreen { monitor_index, region, format } => {
                let req = hcs_vision_proto::CaptureRequest {
                    monitor_index: monitor_index.unwrap_or(0),
                    region: region.as_ref().map(|r| hcs_vision_proto::Region {
                        x: r.x,
                        y: r.y,
                        width: r.width,
                        height: r.height,
                    }),
                    window_title: "".to_string(),
                    include_cursor: false,
                    format: match format.as_deref() {
                        Some("rgb") => hcs_vision_proto::ImageFormat::ImageFormatRgb as i32,
                        Some("rgba") => hcs_vision_proto::ImageFormat::ImageFormatRgba as i32,
                        Some("bgr") => hcs_vision_proto::ImageFormat::ImageFormatBgr as i32,
                        Some("gray") => hcs_vision_proto::ImageFormat::ImageFormatGray as i32,
                        Some("jpeg") => hcs_vision_proto::ImageFormat::ImageFormatJpeg as i32,
                        Some("png") => hcs_vision_proto::ImageFormat::ImageFormatPng as i32,
                        _ => hcs_vision_proto::ImageFormat::ImageFormatBgr as i32,
                    },
                };
                
                match self.client.capture_screen(req).await {
                    Ok(resp) => {
                        let var_name = format!("vision.capture.{}", step.id);
                        self.context.set_var(var_name.clone(), serde_json::json!({
                            "width": resp.width,
                            "height": resp.height,
                            "format": format!("{:?}", resp.format),
                            "timestamp_us": resp.timestamp_us,
                        }));
                        StepResult {
                            step_id: step.id.clone(),
                            status: StepStatus::Completed,
                            output: Some(serde_json::json!({"capture_var": var_name})),
                            error: None,
                            start_time: chrono::Utc::now(),
                            end_time: None,
                            sub_steps: vec![],
                        }
                    }
                    Err(e) => StepResult {
                        step_id: step.id.clone(),
                        status: StepStatus::Failed,
                        output: None,
                        error: Some(e.to_string()),
                        start_time: chrono::Utc::now(),
                        end_time: None,
                        sub_steps: vec![],
                    }
                }
            }
            VisionOperation::DetectObjects { image, class_filter, confidence_threshold, iou_threshold, max_detections } => {
                // Would need to get image from variable
                StepResult {
                    step_id: step.id.clone(),
                    status: StepStatus::Completed,
                    output: Some(serde_json::json!({"detections": []})),
                    error: None,
                    start_time: chrono::Utc::now(),
                    end_time: None,
                    sub_steps: vec![],
                }
            }
            VisionOperation::RecognizeText { image, region, languages, confidence_threshold } => {
                StepResult {
                    step_id: step.id.clone(),
                    status: StepStatus::Completed,
                    output: Some(serde_json::json!({"text_blocks": []})),
                    error: None,
                    start_time: chrono::Utc::now(),
                    end_time: None,
                    sub_steps: vec![],
                }
            }
            VisionOperation::WaitForObject { class_name, confidence_threshold, timeout_ms, monitor_index, region } => {
                let start = Instant::now();
                let timeout = Duration::from_millis(*timeout_ms as u64);
                
                loop {
                    if start.elapsed() >= timeout {
                        return StepResult {
                            step_id: step.id.clone(),
                            status: StepStatus::Failed,
                            output: None,
                            error: Some("Timeout waiting for object".to_string()),
                            start_time: chrono::Utc::now(),
                            end_time: None,
                            sub_steps: vec![],
                        };
                    }

                    // Capture and detect
                    let req = hcs_vision_proto::CaptureRequest {
                        monitor_index: monitor_index.unwrap_or(0),
                        region: region.as_ref().map(|r| hcs_vision_proto::Region {
                            x: r.x, y: r.y, width: r.width, height: r.height,
                        }),
                        window_title: "".to_string(),
                        include_cursor: false,
                        format: hcs_vision_proto::ImageFormat::ImageFormatBgr as i32,
                    };

                    if let Ok(capture) = self.client.capture_screen(req).await {
                        let det_req = hcs_vision_proto::DetectionRequest {
                            image_data: capture.image_data,
                            width: capture.width,
                            height: capture.height,
                            format: capture.format,
                            class_filter: vec![class_name.clone()],
                            confidence_threshold: confidence_threshold.unwrap_or(0.5),
                            iou_threshold: iou_threshold.unwrap_or(0.45),
                            max_detections: max_detections.unwrap_or(10),
                        };

                        if let Ok(det_resp) = self.client.detect_objects(det_req).await {
                            if !det_resp.detections.is_empty() {
                                return StepResult {
                                    step_id: step.id.clone(),
                                    status: StepStatus::Completed,
                                    output: Some(serde_json::json!({"detections": det_resp.detections.len()})),
                                    error: None,
                                    start_time: chrono::Utc::now(),
                                    end_time: None,
                                    sub_steps: vec![],
                                };
                            }
                        }
                    }

                    sleep(Duration::from_millis(100)).await;
                }
            }
            VisionOperation::WaitForText { text_pattern, timeout_ms, monitor_index, region, languages } => {
                StepResult {
                    step_id: step.id.clone(),
                    status: StepStatus::Completed,
                    output: Some(serde_json::json!({"found": false})),
                    error: None,
                    start_time: chrono::Utc::now(),
                    end_time: None,
                    sub_steps: vec![],
                }
            }
        }
    }

    async fn execute_brain(&self, step: &BrainStep) -> StepResult {
        debug!("Executing brain step: {}", step.name);

        match step.operation {
            BrainOperation::ExecuteTree => {
                let tree_id = step.tree_id.as_deref().unwrap_or("default");
                let blackboard = step.blackboard.clone();
                
                match self.client.execute_brain_tree(tree_id, step.tree_definition.clone(), blackboard).await {
                    Ok(result) => StepResult {
                        step_id: step.id.clone(),
                        status: StepStatus::Completed,
                        output: Some(result),
                        error: None,
                        start_time: chrono::Utc::now(),
                        end_time: None,
                        sub_steps: vec![],
                    },
                    Err(e) => StepResult {
                        step_id: step.id.clone(),
                        status: StepStatus::Failed,
                        output: None,
                        error: Some(e.to_string()),
                        start_time: chrono::Utc::now(),
                        end_time: None,
                        sub_steps: vec![],
                    }
                }
            }
            BrainOperation::TickTree => {
                StepResult {
                    step_id: step.id.clone(),
                    status: StepStatus::Completed,
                    output: Some(serde_json::json!({"ticked": true})),
                    error: None,
                    start_time: chrono::Utc::now(),
                    end_time: None,
                    sub_steps: vec![],
                }
            }
            BrainOperation::LoadTree => {
                StepResult {
                    step_id: step.id.clone(),
                    status: StepStatus::Completed,
                    output: Some(serde_json::json!({"loaded": true})),
                    error: None,
                    start_time: chrono::Utc::now(),
                    end_time: None,
                    sub_steps: vec![],
                }
            }
            _ => StepResult {
                step_id: step.id.clone(),
                status: StepStatus::Failed,
                output: None,
                error: Some("Unsupported brain operation".to_string()),
                start_time: chrono::Utc::now(),
                end_time: None,
                sub_steps: vec![],
            }
        }
    }

    async fn execute_adapter(&self, step: &AdapterStep) -> StepResult {
        debug!("Executing adapter step: {} ({:?})", step.name, step.adapter);

        match step.adapter {
            AdapterType::Photoshop => self.execute_photoshop(step).await,
            AdapterType::Chrome => self.execute_chrome(step).await,
            AdapterType::Game => self.execute_game(step).await,
            AdapterType::Window => self.execute_window(step).await,
        }
    }

    async fn execute_photoshop(&self, step: &AdapterStep) -> StepResult {
        match self.client.photoshop_operation(&step.operation, step.parameters.clone()).await {
            Ok(result) => StepResult {
                step_id: step.id.clone(),
                status: StepStatus::Completed,
                output: Some(result),
                error: None,
                start_time: chrono::Utc::now(),
                end_time: None,
                sub_steps: vec![],
            },
            Err(e) => StepResult {
                step_id: step.id.clone(),
                status: StepStatus::Failed,
                output: None,
                error: Some(e.to_string()),
                start_time: chrono::Utc::now(),
                end_time: None,
                sub_steps: vec![],
            }
        }
    }

    async fn execute_chrome(&self, step: &AdapterStep) -> StepResult {
        match self.client.chrome_operation(&step.operation, step.parameters.clone()).await {
            Ok(result) => StepResult {
                step_id: step.id.clone(),
                status: StepStatus::Completed,
                output: Some(result),
                error: None,
                start_time: chrono::Utc::now(),
                end_time: None,
                sub_steps: vec![],
            },
            Err(e) => StepResult {
                step_id: step.id.clone(),
                status: StepStatus::Failed,
                output: None,
                error: Some(e.to_string()),
                start_time: chrono::Utc::now(),
                end_time: None,
                sub_steps: vec![],
            }
        }
    }

    async fn execute_game(&self, step: &AdapterStep) -> StepResult {
        match self.client.game_operation(&step.operation, step.parameters.clone()).await {
            Ok(result) => StepResult {
                step_id: step.id.clone(),
                status: StepStatus::Completed,
                output: Some(result),
                error: None,
                start_time: chrono::Utc::now(),
                end_time: None,
                sub_steps: vec![],
            },
            Err(e) => StepResult {
                step_id: step.id.clone(),
                status: StepStatus::Failed,
                output: None,
                error: Some(e.to_string()),
                start_time: chrono::Utc::now(),
                end_time: None,
                sub_steps: vec![],
            }
        }
    }

    async fn execute_window(&self, step: &AdapterStep) -> StepResult {
        match self.client.window_operation(&step.operation, step.parameters.clone()).await {
            Ok(result) => StepResult {
                step_id: step.id.clone(),
                status: StepStatus::Completed,
                output: Some(result),
                error: None,
                start_time: chrono::Utc::now(),
                end_time: None,
                sub_steps: vec![],
            },
            Err(e) => StepResult {
                step_id: step.id.clone(),
                status: StepStatus::Failed,
                output: None,
                error: Some(e.to_string()),
                start_time: chrono::Utc::now(),
                end_time: None,
                sub_steps: vec![],
            }
        }
    }

    async fn execute_variable(&mut self, step: &VariableStep) -> StepResult {
        debug!("Executing variable step: {}", step.name);

        match &step.operation {
            VariableOperation::Set { name, value } => {
                let resolved = self.resolve_value(value);
                self.context.set_var(name.clone(), resolved);
                StepResult {
                    step_id: step.id.clone(),
                    status: StepStatus::Completed,
                    output: Some(serde_json::json!({"set": name})),
                    error: None,
                    start_time: chrono::Utc::now(),
                    end_time: None,
                    sub_steps: vec![],
                }
            }
            VariableOperation::Get { name } => {
                let value = self.context.get_var(name).cloned().unwrap_or(serde_json::Value::Null);
                StepResult {
                    step_id: step.id.clone(),
                    status: StepStatus::Completed,
                    output: Some(value),
                    error: None,
                    start_time: chrono::Utc::now(),
                    end_time: None,
                    sub_steps: vec![],
                }
            }
            VariableOperation::Delete { name } => {
                self.context.variables.remove(name);
                StepResult {
                    step_id: step.id.clone(),
                    status: StepStatus::Completed,
                    output: Some(serde_json::json!({"deleted": name})),
                    error: None,
                    start_time: chrono::Utc::now(),
                    end_time: None,
                    sub_steps: vec![],
                }
            }
            VariableOperation::Template { template, output_var } => {
                let resolved = self.context.resolve_template(template);
                self.context.set_var(output_var.clone(), serde_json::json!(resolved));
                StepResult {
                    step_id: step.id.clone(),
                    status: StepStatus::Completed,
                    output: Some(serde_json::json!({"template": resolved})),
                    error: None,
                    start_time: chrono::Utc::now(),
                    end_time: None,
                    sub_steps: vec![],
                }
            }
            VariableOperation::JsonPath { json_var, path, output_var } => {
                if let Some(json_value) = self.context.get_var(json_var) {
                    // Simplified JSONPath - would use a proper library
                    let result = json_value.clone();
                    self.context.set_var(output_var.clone(), result);
                    StepResult {
                        step_id: step.id.clone(),
                        status: StepStatus::Completed,
                        output: Some(serde_json::json!({"path": path})),
                        error: None,
                        start_time: chrono::Utc::now(),
                        end_time: None,
                        sub_steps: vec![],
                    }
                } else {
                    StepResult {
                        step_id: step.id.clone(),
                        status: StepStatus::Failed,
                        output: None,
                        error: Some(format!("Variable '{}' not found", json_var)),
                        start_time: chrono::Utc::now(),
                        end_time: None,
                        sub_steps: vec![],
                    }
                }
            }
        }
    }

    fn resolve_value(&self, value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::String(s) => {
                let resolved = self.context.resolve_template(s);
                // Try to parse as JSON
                if resolved.starts_with('{') || resolved.starts_with('[') {
                    serde_json::from_str(&resolved).unwrap_or(serde_json::json!(resolved))
                } else {
                    serde_json::json!(resolved)
                }
            }
            serde_json::Value::Object(obj) => {
                let mut result = serde_json::Map::new();
                for (k, v) in obj {
                    result.insert(k.clone(), self.resolve_value(v));
                }
                serde_json::Value::Object(result)
            }
            serde_json::Value::Array(arr) => {
                let result: Vec<_> = arr.iter().map(|v| self.resolve_value(v)).collect();
                serde_json::Value::Array(result)
            }
            _ => value.clone(),
        }
    }

    async fn execute_wait(&mut self, step: &WaitStep) -> StepResult {
        debug!("Executing wait step: {}", step.name);

        let timeout = Duration::from_millis(step.timeout_ms as u64);
        let poll_interval = Duration::from_millis(step.poll_interval_ms.unwrap_or(100) as u64);
        let start = Instant::now();

        loop {
            if start.elapsed() >= timeout {
                return StepResult {
                    step_id: step.id.clone(),
                    status: StepStatus::Failed,
                    output: None,
                    error: Some("Wait timeout".to_string()),
                    start_time: chrono::Utc::now(),
                    end_time: None,
                    sub_steps: vec![],
                };
            }

            let condition_result = match step.condition_type {
                WaitConditionType::Expression => {
                    self.evaluate_condition(&step.condition).await.unwrap_or(false)
                }
                WaitConditionType::VisionObject => {
                    // Would check vision
                    false
                }
                WaitConditionType::VisionText => {
                    // Would check vision
                    false
                }
                WaitConditionType::WindowExists => {
                    // Would check window
                    false
                }
                WaitConditionType::ProcessExists => {
                    // Would check process
                    false
                }
            };

            if condition_result {
                return StepResult {
                    step_id: step.id.clone(),
                    status: StepStatus::Completed,
                    output: Some(serde_json::json!({"waited_ms": start.elapsed().as_millis()})),
                    error: None,
                    start_time: chrono::Utc::now(),
                    end_time: None,
                    sub_steps: vec![],
                };
            }

            sleep(poll_interval).await;
        }
    }

    async fn execute_log(&self, step: &LogStep) -> StepResult {
        let message = self.context.resolve_template(&step.message);
        
        match step.level {
            LogLevel::Trace => tracing::trace!("{}", message),
            LogLevel::Debug => tracing::debug!("{}", message),
            LogLevel::Info => tracing::info!("{}", message),
            LogLevel::Warn => tracing::warn!("{}", message),
            LogLevel::Error => tracing::error!("{}", message),
        }

        StepResult {
            step_id: step.id.clone(),
            status: StepStatus::Completed,
            output: Some(serde_json::json!({"message": message})),
            error: None,
            start_time: chrono::Utc::now(),
            end_time: None,
            sub_steps: vec![],
        }
    }
}

fn json_to_starlark(value: &serde_json::Value) -> Result<starlark::values::Value> {
    use starlark::values::Value;
    Ok(match value {
        serde_json::Value::Null => Value::new_none(),
        serde_json::Value::Bool(b) => Value::new_bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::new_int(i)
            } else if let Some(f) = n.as_f64() {
                Value::new_float(f)
            } else {
                Value::new_int(0)
            }
        }
        serde_json::Value::String(s) => Value::new_string(s.as_str()),
        serde_json::Value::Array(arr) => {
            let list: Vec<Value> = arr.iter().map(|v| json_to_starlark(v).unwrap()).collect();
            Value::new_list(list)
        }
        serde_json::Value::Object(obj) => {
            let dict = starlark::values::dict::Dict::new();
            for (k, v) in obj {
                dict.insert(Value::new_string(k), json_to_starlark(v)?)?;
            }
            Value::new_dict(dict)
        }
    })
}