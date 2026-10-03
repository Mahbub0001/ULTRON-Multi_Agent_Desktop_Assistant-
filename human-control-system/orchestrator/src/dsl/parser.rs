use crate::dsl::types::*;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct DslParser {
    include_paths: Vec<PathBuf>,
}

impl DslParser {
    pub fn new() -> Self {
        Self {
            include_paths: vec![PathBuf::from(".")],
        }
    }

    pub fn with_include_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.include_paths = paths;
        self
    }

    pub fn parse_file<P: AsRef<Path>>(&self, path: P) -> Result<TaskDefinition> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read task file: {}", path.display()))?;

        let mut task = self.parse_content(&content)?;
        
        // Resolve imports
        if !task.imports.is_empty() {
            let base_dir = path.parent().unwrap_or(Path::new("."));
            for import_path in &task.imports {
                let imported = self.parse_import(base_dir, import_path)?;
                self.merge_import(&mut task, imported)?;
            }
        }

        Ok(task)
    }

    pub fn parse_content(&self, content: &str) -> Result<TaskDefinition> {
        // Try YAML first
        if let Ok(task) = serde_yaml::from_str::<TaskDefinition>(content) {
            return Ok(task);
        }

        // Try JSON
        if let Ok(task) = serde_json::from_str::<TaskDefinition>(content) {
            return Ok(task);
        }

        // Try Starlark (basic support)
        self.parse_starlark(content)
    }

    fn parse_import(&self, base_dir: &Path, import_path: &str) -> Result<TaskDefinition> {
        // Try relative to base dir first
        let mut full_path = base_dir.join(import_path);
        
        if !full_path.exists() {
            // Try include paths
            for include_path in &self.include_paths {
                let candidate = include_path.join(import_path);
                if candidate.exists() {
                    full_path = candidate;
                    break;
                }
            }
        }

        if !full_path.exists() {
            anyhow::bail!("Import not found: {}", import_path);
        }

        self.parse_file(&full_path)
    }

    fn merge_import(&self, task: &mut TaskDefinition, imported: TaskDefinition) -> Result<()> {
        // Merge variables (imported vars can be overridden)
        for (key, value) in imported.variables {
            task.variables.entry(key).or_insert(value);
        }

        // Merge macros (imported macros can be overridden)
        for (key, value) in imported.macros {
            task.macros.entry(key).or_insert(value);
        }

        // Prepend imported steps (so they run first)
        let mut steps = imported.steps;
        steps.append(&mut task.steps);
        task.steps = steps;

        Ok(())
    }

    fn parse_starlark(&self, content: &str) -> Result<TaskDefinition> {
        // Basic Starlark parsing - for now just return an error
        // Full Starlark support would require a proper interpreter
        anyhow::bail!("Starlark parsing not yet implemented. Use YAML or JSON format.");
    }

    pub fn validate(&self, task: &TaskDefinition) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        // Check for duplicate step IDs
        let mut step_ids = HashMap::new();
        self.collect_step_ids(&task.steps, &mut step_ids);
        
        for (id, count) in step_ids {
            if count > 1 {
                warnings.push(format!("Duplicate step ID: {}", id));
            }
        }

        // Check for undefined macro references
        self.check_macro_references(&task.steps, &task.macros, &mut warnings);

        // Check for undefined variables
        self.check_variable_references(&task.steps, &task.variables, &mut warnings);

        Ok(warnings)
    }

    fn collect_step_ids(&self, steps: &[Step], ids: &mut HashMap<String, usize>) {
        for step in steps {
            let id = match step {
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
            };
            *ids.entry(id.clone()).or_insert(0) += 1;

            // Recurse into nested steps
            match step {
                Step::Sequential(s) | Step::Parallel(s) => {
                    self.collect_step_ids(&s.steps, ids);
                }
                Step::Conditional(s) => {
                    self.collect_step_ids(&[s.then_branch.as_ref().clone()], ids);
                    if let Some(else_branch) = &s.else_branch {
                        self.collect_step_ids(&[else_branch.as_ref().clone()], ids);
                    }
                }
                Step::Loop(s) => {
                    self.collect_step_ids(&[s.body.as_ref().clone()], ids);
                }
                _ => {}
            }
        }
    }

    fn check_macro_references(&self, steps: &[Step], macros: &HashMap<String, MacroDef>, warnings: &mut Vec<String>) {
        for step in steps {
            if let Step::Macro(m) = step {
                if !macros.contains_key(&m.macro_name) {
                    warnings.push(format!("Step '{}' references undefined macro: {}", m.id, m.macro_name));
                }
            }

            // Recurse
            match step {
                Step::Sequential(s) | Step::Parallel(s) => {
                    self.check_macro_references(&s.steps, macros, warnings);
                }
                Step::Conditional(s) => {
                    self.check_macro_references(&[s.then_branch.as_ref().clone()], macros, warnings);
                    if let Some(else_branch) = &s.else_branch {
                        self.check_macro_references(&[else_branch.as_ref().clone()], macros, warnings);
                    }
                }
                Step::Loop(s) => {
                    self.check_macro_references(&[s.body.as_ref().clone()], macros, warnings);
                }
                _ => {}
            }
        }
    }

    fn check_variable_references(&self, steps: &[Step], variables: &HashMap<String, VariableDef>, warnings: &mut Vec<String>) {
        // This is a simplified check - in reality we'd need to parse expressions
        for step in steps {
            self.check_step_variables(step, variables, warnings);
        }
    }

    fn check_step_variables(&self, step: &Step, variables: &HashMap<String, VariableDef>, warnings: &mut Vec<String>) {
        match step {
            Step::Action(s) => {
                for (_, value) in &s.parameters {
                    if let serde_json::Value::String(str_val) = value {
                        self.check_template_vars(str_val, variables, warnings);
                    }
                }
            }
            Step::Macro(s) => {
                for (_, value) in &s.parameters {
                    if let serde_json::Value::String(str_val) = value {
                        self.check_template_vars(str_val, variables, warnings);
                    }
                }
            }
            Step::Variable(s) => {
                if let VariableOperation::Template { template, .. } = &s.operation {
                    self.check_template_vars(template, variables, warnings);
                }
            }
            Step::Wait(s) => {
                self.check_template_vars(&s.condition, variables, warnings);
            }
            Step::Conditional(s) => {
                self.check_template_vars(&s.condition, variables, warnings);
            }
            Step::Loop(s) => {
                if let Some(coll) = &s.collection {
                    self.check_template_vars(coll, variables, warnings);
                }
                if let Some(cond) = &s.condition {
                    self.check_template_vars(cond, variables, warnings);
                }
            }
            _ => {}
        }

        // Recurse into nested steps
        match step {
            Step::Sequential(s) | Step::Parallel(s) => {
                for nested in &s.steps {
                    self.check_step_variables(nested, variables, warnings);
                }
            }
            Step::Conditional(s) => {
                self.check_step_variables(&s.then_branch, variables, warnings);
                if let Some(else_branch) = &s.else_branch {
                    self.check_step_variables(else_branch, variables, warnings);
                }
            }
            Step::Loop(s) => {
                self.check_step_variables(&s.body, variables, warnings);
            }
            _ => {}
        }
    }

    fn check_template_vars(&self, template: &str, variables: &HashMap<String, VariableDef>, warnings: &mut Vec<String>) {
        // Simple regex to find ${var} patterns
        let re = regex::Regex::new(r"\$\{([^}]+)\}").unwrap();
        for cap in re.captures_iter(template) {
            if let Some(var_name) = cap.get(1) {
                let name = var_name.as_str();
                if !variables.contains_key(name) && !is_builtin_var(name) {
                    warnings.push(format!("Template references undefined variable: ${{{}}}", name));
                }
            }
        }
    }
}

fn is_builtin_var(name: &str) -> bool {
    matches!(name, 
        "HCS_VERSION" | "HCS_OS" | "HCS_TASK_ID" | "HCS_STEP_ID" | 
        "true" | "false" | "null"
    )
}

impl Default for DslParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_yaml_task() {
        let yaml = r#"
name: "Test Task"
version: "1.0"
description: "A test task"
variables:
  count:
    type: "int"
    default: 5
steps:
  - type: action
    id: "step1"
    name: "Press Enter"
    action:
      kind: "key_press"
      code: 13
"#;
        let parser = DslParser::new();
        let task = parser.parse_content(yaml).unwrap();
        assert_eq!(task.name, "Test Task");
        assert_eq!(task.steps.len(), 1);
    }

    #[test]
    fn test_parse_sequential() {
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
        if let Step::Sequential(seq) = &task.steps[0] {
            assert_eq!(seq.steps.len(), 2);
        } else {
            panic!("Expected sequential step");
        }
    }
}