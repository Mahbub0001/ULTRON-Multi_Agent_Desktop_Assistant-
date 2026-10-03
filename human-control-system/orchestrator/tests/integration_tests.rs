use hcs_orchestrator::dsl::{DslParser, Interpreter, InterpreterConfig, TaskDefinition};
use hcs_orchestrator::recorder::{Recorder, RecorderConfig};
use std::collections::HashMap;

#[test]
fn test_parse_simple_yaml_task() {
    let yaml = r#"
name: "Test Task"
version: "1.0"
description: "A simple test task"
variables:
  count:
    type: "int"
    default: 5
    description: "Number of iterations"
steps:
  - type: action
    id: "step1"
    name: "Press Enter"
    action:
      kind: "key_press"
      code: 13
      delay_ms: 50
"#;
    let parser = DslParser::new();
    let task = parser.parse_content(yaml).unwrap();
    
    assert_eq!(task.name, "Test Task");
    assert_eq!(task.version, "1.0");
    assert_eq!(task.steps.len(), 1);
    
    if let hcs_orchestrator::dsl::types::Step::Action(action_step) = &task.steps[0] {
        assert_eq!(action_step.id, "step1");
        assert_eq!(action_step.name, "Press Enter");
    } else {
        panic!("Expected action step");
    }
}

#[test]
fn test_parse_sequential_task() {
    let yaml = r#"
name: "Sequential Task"
version: "1.0"
steps:
  - type: sequential
    id: "seq1"
    name: "Sequence"
    steps:
      - type: action
        id: "step1"
        name: "Step 1"
        action:
          kind: "key_press"
          code: 13
      - type: action
        id: "step2"
        name: "Step 2"
        action:
          kind: "key_press"
          code: 32
"#;
    let parser = DslParser::new();
    let task = parser.parse_content(yaml).unwrap();
    
    assert_eq!(task.steps.len(), 1);
    if let hcs_orchestrator::dsl::types::Step::Sequential(seq) = &task.steps[0] {
        assert_eq!(seq.steps.len(), 2);
    } else {
        panic!("Expected sequential step");
    }
}

#[test]
fn test_parse_parallel_task() {
    let yaml = r#"
name: "Parallel Task"
version: "1.0"
steps:
  - type: parallel
    id: "par1"
    name: "Parallel"
    fail_fast: true
    steps:
      - type: action
        id: "step1"
        name: "Step 1"
        action:
          kind: "key_press"
          code: 13
      - type: action
        id: "step2"
        name: "Step 2"
        action:
          kind: "mouse_click"
          button: "left"
"#;
    let parser = DslParser::new();
    let task = parser.parse_content(yaml).unwrap();
    
    assert_eq!(task.steps.len(), 1);
    if let hcs_orchestrator::dsl::types::Step::Parallel(par) = &task.steps[0] {
        assert_eq!(par.steps.len(), 2);
        assert!(par.fail_fast);
    } else {
        panic!("Expected parallel step");
    }
}

#[test]
fn test_parse_conditional_task() {
    let yaml = r#"
name: "Conditional Task"
version: "1.0"
steps:
  - type: conditional
    id: "cond1"
    name: "If condition"
    condition: "${count} > 0"
    then_branch:
      type: action
      id: "then_step"
      name: "Then branch"
      action:
        kind: "key_press"
        code: 13
    else_branch:
      type: action
      id: "else_step"
      name: "Else branch"
      action:
        kind: "key_press"
        code: 32
"#;
    let parser = DslParser::new();
    let task = parser.parse_content(yaml).unwrap();
    
    assert_eq!(task.steps.len(), 1);
    if let hcs_orchestrator::dsl::types::Step::Conditional(cond) = &task.steps[0] {
        assert_eq!(cond.condition, "${count} > 0");
        assert!(cond.else_branch.is_some());
    } else {
        panic!("Expected conditional step");
    }
}

#[test]
fn test_parse_loop_task() {
    let yaml = r#"
name: "Loop Task"
version: "1.0"
variables:
  items:
    type: "list"
    default: "[1, 2, 3]"
steps:
  - type: loop
    id: "loop1"
    name: "For each item"
    loop_type: "for_each"
    iterator: "item"
    collection: "${items}"
    body:
      type: action
      id: "loop_body"
      name: "Process item"
      action:
        kind: "type_text"
        text: "${item}"
"#;
    let parser = DslParser::new();
    let task = parser.parse_content(yaml).unwrap();
    
    assert_eq!(task.steps.len(), 1);
    if let hcs_orchestrator::dsl::types::Step::Loop(loop_step) = &task.steps[0] {
        assert_eq!(loop_step.loop_type, hcs_orchestrator::dsl::types::LoopType::ForEach);
        assert_eq!(loop_step.iterator, Some("item".to_string()));
        assert_eq!(loop_step.collection, Some("${items}".to_string()));
    } else {
        panic!("Expected loop step");
    }
}

#[test]
fn test_parse_macro_definition() {
    let yaml = r#"
name: "Task with Macro"
version: "1.0"
macros:
  my_macro:
    name: "My Macro"
    description: "A test macro"
    parameters:
      text:
        type: "string"
        required: true
    steps:
      - type: action
        id: "macro_step"
        name: "Type text"
        action:
          kind: "type_text"
          text: "${text}"
steps:
  - type: macro
    id: "run_macro"
    name: "Run macro"
    macro_name: "my_macro"
    parameters:
      text: "Hello World"
"#;
    let parser = DslParser::new();
    let task = parser.parse_content(yaml).unwrap();
    
    assert_eq!(task.macros.len(), 1);
    assert!(task.macros.contains_key("my_macro"));
    assert_eq!(task.steps.len(), 1);
}

#[test]
fn test_validation_warnings() {
    let yaml = r#"
name: "Task with Issues"
version: "1.0"
steps:
  - type: macro
    id: "step1"
    name: "Run undefined macro"
    macro_name: "undefined_macro"
"#;
    let parser = DslParser::new();
    let task = parser.parse_content(yaml).unwrap();
    let warnings = parser.validate(&task).unwrap();
    
    assert!(!warnings.is_empty());
    assert!(warnings.iter().any(|w| w.contains("undefined macro")));
}

#[test]
fn test_variable_resolution() {
    let mut context = hcs_orchestrator::dsl::types::ExecutionContext::new("test-task".to_string());
    context.set_var("name".to_string(), serde_json::json!("World"));
    context.set_var("count".to_string(), serde_json::json!(5));
    
    let resolved = context.resolve_template("Hello ${name}, count is ${count}");
    assert_eq!(resolved, "Hello World, count is 5");
}

#[test]
fn test_interpreter_config_defaults() {
    let config = InterpreterConfig::default();
    assert_eq!(config.max_execution_time_ms, 300000);
    assert_eq!(config.max_loop_iterations, 10000);
    assert_eq!(config.max_recursion_depth, 100);
    assert_eq!(config.default_step_timeout_ms, 10000);
}

#[test]
fn test_recorder_config_defaults() {
    let config = RecorderConfig::default();
    assert!(config.capture_keyboard);
    assert!(config.capture_mouse);
    assert!(config.capture_delays);
    assert!(config.max_duration.is_none());
    assert!(config.stop_key.is_none());
}

#[test]
fn test_task_definition_serialization() {
    let task = TaskDefinition {
        name: "Serialization Test".to_string(),
        version: "1.0".to_string(),
        description: "Test serialization".to_string(),
        variables: HashMap::new(),
        steps: vec![],
        macros: HashMap::new(),
        imports: vec![],
    };
    
    let yaml = serde_yaml::to_string(&task).unwrap();
    assert!(yaml.contains("Serialization Test"));
    
    let json = serde_json::to_string(&task).unwrap();
    assert!(json.contains("Serialization Test"));
}

#[tokio::test]
async fn test_interpreter_creation() {
    // This test would require a mock gRPC client
    // For now, just verify the types compile
    let _config = InterpreterConfig::default();
}