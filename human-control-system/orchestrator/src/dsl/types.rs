use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Step {
    Action(ActionStep),
    Macro(MacroStep),
    Sequential(SequentialStep),
    Parallel(ParallelStep),
    Conditional(ConditionalStep),
    Loop(LoopStep),
    Vision(VisionStep),
    Brain(BrainStep),
    Adapter(AdapterStep),
    Variable(VariableStep),
    Wait(WaitStep),
    Log(LogStep),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionStep {
    pub id: String,
    pub name: String,
    pub action: ActionType,
    pub parameters: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u32>,
    #[serde(default)]
    pub retry: Option<RetryPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActionType {
    KeyDown { code: u32, scan_code: Option<u32>, extended: Option<bool> },
    KeyUp { code: u32, scan_code: Option<u32>, extended: Option<bool> },
    KeyPress { code: u32, scan_code: Option<u32>, extended: Option<bool>, delay_ms: Option<u32> },
    MouseMove { x: i32, y: i32, absolute: Option<bool> },
    MouseClick { button: MouseButton, x: Option<i32>, y: Option<i32>, count: Option<u32> },
    MouseDown { button: MouseButton, x: Option<i32>, y: Option<i32> },
    MouseUp { button: MouseButton, x: Option<i32>, y: Option<i32> },
    MouseScroll { dx: i32, dy: i32 },
    Delay { microseconds: u64 },
    TypeText { text: String, delay_ms: Option<u32> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
    WheelUp,
    WheelDown,
    WheelLeft,
    WheelRight,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MacroStep {
    pub id: String,
    pub name: String,
    pub macro_name: String,
    pub parameters: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u32>,
    #[serde(default)]
    pub retry: Option<RetryPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SequentialStep {
    pub id: String,
    pub name: String,
    pub steps: Vec<Step>,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParallelStep {
    pub id: String,
    pub name: String,
    pub steps: Vec<Step>,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u32>,
    #[serde(default = "default_true")]
    pub fail_fast: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConditionalStep {
    pub id: String,
    pub name: String,
    pub condition: String,
    pub then_branch: Box<Step>,
    #[serde(default)]
    pub else_branch: Option<Box<Step>>,
    #[serde(default)]
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoopStep {
    pub id: String,
    pub name: String,
    pub loop_type: LoopType,
    pub iterator: Option<String>,
    pub collection: Option<String>,
    pub condition: Option<String>,
    pub count: Option<u32>,
    pub body: Box<Step>,
    #[serde(default)]
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LoopType {
    ForEach,
    While,
    Count,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisionStep {
    pub id: String,
    pub name: String,
    pub operation: VisionOperation,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u32>,
    #[serde(default)]
    pub retry: Option<RetryPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VisionOperation {
    CaptureScreen {
        monitor_index: Option<i32>,
        region: Option<Region>,
        format: Option<String>,
    },
    DetectObjects {
        image: Option<String>, // variable reference
        class_filter: Option<Vec<String>>,
        confidence_threshold: Option<f32>,
        iou_threshold: Option<f32>,
        max_detections: Option<i32>,
    },
    RecognizeText {
        image: Option<String>,
        region: Option<Region>,
        languages: Option<Vec<String>>,
        confidence_threshold: Option<f32>,
    },
    WaitForObject {
        class_name: String,
        confidence_threshold: Option<f32>,
        timeout_ms: u32,
        monitor_index: Option<i32>,
        region: Option<Region>,
    },
    WaitForText {
        text_pattern: String,
        timeout_ms: u32,
        monitor_index: Option<i32>,
        region: Option<Region>,
        languages: Option<Vec<String>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BrainStep {
    pub id: String,
    pub name: String,
    pub tree_id: Option<String>,
    pub tree_definition: Option<String>,
    pub blackboard: HashMap<String, String>,
    pub operation: BrainOperation,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BrainOperation {
    ExecuteTree,
    TickTree,
    LoadTree,
    UnloadTree,
    GetStatus,
    GetVisualization { format: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdapterStep {
    pub id: String,
    pub name: String,
    pub adapter: AdapterType,
    pub operation: String,
    pub parameters: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u32>,
    #[serde(default)]
    pub retry: Option<RetryPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AdapterType {
    Photoshop,
    Chrome,
    Game,
    Window,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariableStep {
    pub id: String,
    pub name: String,
    pub operation: VariableOperation,
    #[serde(default)]
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VariableOperation {
    Set { name: String, value: serde_json::Value },
    Get { name: String },
    Delete { name: String },
    Template { template: String, output_var: String },
    JsonPath { json_var: String, path: String, output_var: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WaitStep {
    pub id: String,
    pub name: String,
    pub condition: String,
    pub timeout_ms: u32,
    pub poll_interval_ms: Option<u32>,
    #[serde(default)]
    pub condition_type: WaitConditionType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WaitConditionType {
    Expression,
    VisionObject,
    VisionText,
    WindowExists,
    ProcessExists,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogStep {
    pub id: String,
    pub name: String,
    pub level: LogLevel,
    pub message: String,
    #[serde(default)]
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub delay_ms: u32,
    #[serde(default = "default_backoff")]
    pub backoff_multiplier: f32,
    #[serde(default)]
    pub max_delay_ms: Option<u32>,
    #[serde(default)]
    pub retry_on_errors: Vec<String>,
}

fn default_backoff() -> f32 {
    2.0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TaskDefinition {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub variables: HashMap<String, VariableDef>,
    pub steps: Vec<Step>,
    #[serde(default)]
    pub macros: HashMap<String, MacroDef>,
    #[serde(default)]
    pub imports: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariableDef {
    pub var_type: String,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MacroDef {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub parameters: HashMap<String, VariableDef>,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionContext {
    pub variables: HashMap<String, serde_json::Value>,
    pub task_id: String,
    pub step_results: HashMap<String, StepResult>,
    pub loop_counters: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepResult {
    pub step_id: String,
    pub status: StepStatus,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub sub_steps: Vec<StepResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

impl Default for StepStatus {
    fn default() -> Self {
        StepStatus::Pending
    }
}

impl ExecutionContext {
    pub fn new(task_id: String) -> Self {
        Self {
            variables: HashMap::new(),
            task_id,
            step_results: HashMap::new(),
            loop_counters: HashMap::new(),
        }
    }

    pub fn get_var(&self, name: &str) -> Option<&serde_json::Value> {
        self.variables.get(name)
    }

    pub fn set_var(&mut self, name: String, value: serde_json::Value) {
        self.variables.insert(name, value);
    }

    pub fn resolve_template(&self, template: &str) -> String {
        let mut result = template.to_string();
        for (key, value) in &self.variables {
            let placeholder = format!("${{{}}}", key);
            let replacement = match value {
                serde_json::Value::String(s) => s.clone(),
                _ => value.to_string(),
            };
            result = result.replace(&placeholder, &replacement);
        }
        result
    }
}