# Task DSL Reference

## Overview

The Task DSL (Domain Specific Language) is a YAML-based language for defining automation tasks in the Human Control System. It supports sequential/parallel execution, conditionals, loops, variables, macros, and integration with all HCS services.

## File Structure

```yaml
# task.yaml
name: "My Automation Task"
version: "1.0.0"
description: "Description of what this task does"

# Variable definitions with types and defaults
variables:
  target_app:
    type: "string"
    default: "notepad"
    description: "Application to automate"
    required: true
  repeat_count:
    type: "int"
    default: 3
    description: "Number of repetitions"
  delay_ms:
    type: "int"
    default: 100
    description: "Delay between actions"

# Reusable macro definitions
macros:
  type_text:
    description: "Type text with human-like delays"
    parameters:
      text:
        type: "string"
        required: true
      wpm:
        type: "int"
        default: 60
    steps:
      - type: "action"
        name: "Type {{text}}"
        action: "type"
        parameters:
          text: "{{text}}"
          wpm: "{{wpm}}"

# Main task steps
steps:
  - id: "launch_app"
    name: "Launch {{target_app}}"
    type: "macro"
    macro: "launch_app"
    parameters:
      app: "{{target_app}}"

  - id: "wait_ready"
    name: "Wait for app ready"
    type: "wait"
    condition: "window_exists('{{target_app}}')"
    timeout_ms: 10000

  - id: "main_loop"
    name: "Main automation loop"
    type: "loop"
    loop:
      type: "count"
      count: "{{repeat_count}}"
    steps:
      - id: "type_hello"
        name: "Type greeting"
        type: "macro"
        macro: "type_text"
        parameters:
          text: "Hello from HCS! Iteration {{loop.index}}"
          wpm: 80

      - id: "press_enter"
        name: "Press Enter"
        type: "action"
        action: "key"
        parameters:
          key: "Enter"
          state: "tap"

      - id: "delay"
        name: "Delay between iterations"
        type: "action"
        action: "delay"
        parameters:
          ms: "{{delay_ms}}"

  - id: "save_file"
    name: "Save file"
    type: "macro"
    macro: "save_file"
    condition: "variable('auto_save') == true"

  - id: "cleanup"
    name: "Close application"
    type: "macro"
    macro: "close_app"
    parameters:
      app: "{{target_app}}"
```

## Core Concepts

### Variables

Variables are defined in the `variables` section and referenced with `{{variable_name}}` syntax.

**Types:**
- `string` - Text values
- `int` - Integer numbers
- `float` - Floating point numbers
- `bool` - Boolean (true/false)
- `list` - Array of values
- `map` - Key-value object

**Variable Definition:**
```yaml
variables:
  var_name:
    type: "string"
    default: "default_value"
    description: "Human-readable description"
    required: false  # If true, must be provided at runtime
```

**Runtime Override:**
```bash
hcs run task.yaml --var target_app=chrome --var repeat_count=5
```

### Steps

Steps are the executable units of a task. Each step has:
- `id`: Unique identifier (auto-generated if omitted)
- `name`: Human-readable name
- `type`: Step type (see below)
- `condition`: Optional Starlark expression to skip step
- `loop`: Optional loop configuration
- `timeout_ms`: Step timeout (default: 30000)
- `retry`: Optional retry policy

### Step Types

#### 1. Action Step (`action`)
Execute a single input action.

```yaml
- id: "click_button"
  type: "action"
  action: "mouse"  # or "key", "delay", "type"
  parameters:
    button: "left"
    x: 100
    y: 200
    clicks: 1
```

**Action Types:**

| Action | Parameters | Description |
|--------|------------|-------------|
| `key` | `key`, `state` (down/up/tap), `modifiers` | Keyboard key |
| `mouse` | `x`, `y`, `dx`, `dy`, `button`, `clicks`, `absolute` | Mouse move/click |
| `delay` | `ms` or `us` | Wait |
| `type` | `text`, `wpm`, `variance` | Type text with human timing |
| `shortcut` | `keys` (array) | Key combination (e.g., ["ctrl", "c"]) |

**Examples:**
```yaml
# Key press
- type: "action"
  action: "key"
  parameters:
    key: "F5"
    state: "tap"

# Key combination (Ctrl+S)
- type: "action"
  action: "shortcut"
  parameters:
    keys: ["ctrl", "s"]

# Type text
- type: "action"
  action: "type"
  parameters:
    text: "Hello World"
    wpm: 60

# Mouse click at coordinates
- type: "action"
  action: "mouse"
  parameters:
    x: 500
    y: 300
    button: "left"
    clicks: 1
    absolute: true
```

#### 2. Macro Step (`macro`)
Execute a predefined or inline macro.

```yaml
- id: "copy_paste"
  type: "macro"
  macro: "copy_paste"  # References macros.copy_paste
  parameters:
    delay_ms: 200
```

**Inline Macro:**
```yaml
- id: "custom_sequence"
  type: "macro"
  macro:
    name: "inline_macro"
    steps:
      - type: "action"
        action: "key"
        parameters: { key: "ctrl", state: "down" }
      - type: "action"
        action: "key"
        parameters: { key: "a", state: "tap" }
      - type: "action"
        action: "key"
        parameters: { key: "ctrl", state: "up" }
```

#### 3. Sequential Block (`sequential`)
Execute steps in order (default behavior).

```yaml
- id: "setup_sequence"
  type: "sequential"
  steps:
    - type: "action" ...
    - type: "macro" ...
```

#### 4. Parallel Block (`parallel`)
Execute steps concurrently.

```yaml
- id: "parallel_actions"
  type: "parallel"
  steps:
    - id: "task_a"
      type: "macro"
      macro: "macro_a"
    - id: "task_b"
      type: "macro"
      macro: "macro_b"
  # Wait for all to complete (default) or first (race mode)
  wait_for: "all"  # or "first", "majority"
```

#### 5. Conditional Step (`conditional`)
If/else branching.

```yaml
- id: "check_and_act"
  type: "conditional"
  condition: "vision_detect('button.png') != null"
  steps:  # Then branch
    - type: "action"
      action: "mouse"
      parameters: { x: 100, y: 200, button: "left" }
  else_steps:  # Else branch (optional)
    - type: "log"
      message: "Button not found, trying alternative"
    - type: "macro"
      macro: "alternative_flow"
```

#### 6. Loop Step (`loop`)
For/while loops.

**Count Loop:**
```yaml
- id: "repeat_5_times"
  type: "loop"
  loop:
    type: "count"
    count: 5
    iterator: "i"  # Variable name for iteration (0-indexed)
  steps:
    - type: "log"
      message: "Iteration {{i}}"
```

**For-Each Loop:**
```yaml
- id: "process_items"
  type: "loop"
  loop:
    type: "for_each"
    collection: "{{item_list}}"  # Variable containing list
    iterator: "item"  # Variable name for current item
  steps:
    - type: "action"
      action: "type"
      parameters:
        text: "Processing {{item}}"
```

**While Loop:**
```yaml
- id: "wait_for_condition"
  type: "loop"
  loop:
    type: "while"
    condition: "not window_exists('dialog')"
    max_iterations: 100  # Safety limit
  steps:
    - type: "action"
      action: "delay"
      parameters: { ms: 100 }
```

#### 7. Vision Step (`vision`)
Computer vision operations.

```yaml
- id: "find_button"
  type: "vision"
  action: "detect"  # or "ocr", "capture"
  parameters:
    model: "yolov8n"
    classes: ["button", "icon"]
    confidence: 0.7
    region: { x: 0, y: 0, width: 1920, height: 1080 }
  output: "button_location"  # Variable to store result
```

**Vision Actions:**
| Action | Parameters | Output |
|--------|------------|--------|
| `detect` | `model`, `classes`, `confidence`, `region` | List of detections |
| `ocr` | `region`, `languages`, `engine` | Text blocks |
| `capture` | `monitor`, `region`, `format` | Image data |

#### 8. Brain Step (`brain`)
Execute behavior tree.

```yaml
- id: "decide_action"
  type: "brain"
  action: "execute"  # or "tick", "load"
  parameters:
    tree_id: "combat_ai"
    blackboard:
      health: "{{current_health}}"
      ammo: "{{current_ammo}}"
  output: "decision"
```

#### 9. Adapter Step (`adapter`)
Application-specific operations.

```yaml
- id: "chrome_navigate"
  type: "adapter"
  service: "chrome"  # photoshop, chrome, game, window
  action: "navigate"
  parameters:
    url: "https://example.com"
    wait_until: "networkidle"

- id: "ps_adjustment"
  type: "adapter"
  service: "photoshop"
  action: "apply_adjustment"
  parameters:
    document_id: "{{doc_id}}"
    type: "brightness_contrast"
    brightness: 20
    contrast: 10
```

#### 10. Variable Step (`variable`)
Set/get variables at runtime.

```yaml
- id: "set_counter"
  type: "variable"
  action: "set"
  name: "counter"
  value: "{{counter + 1}}"

- id: "get_config"
  type: "variable"
  action: "get"
  name: "config_value"
  output: "retrieved_value"
```

#### 11. Wait Step (`wait`)
Wait for condition with polling.

```yaml
- id: "wait_for_window"
  type: "wait"
  condition: "window_exists('Save As')"
  timeout_ms: 5000
  poll_interval_ms: 200
```

#### 12. Log Step (`log`)
Structured logging.

```yaml
- id: "log_progress"
  type: "log"
  level: "info"  # debug, info, warn, error
  message: "Completed iteration {{loop.index}} of {{repeat_count}}"
  fields:
    iteration: "{{loop.index}}"
    remaining: "{{repeat_count - loop.index - 1}}"
```

---

## Conditions

Conditions are Starlark (Python-like) expressions that evaluate to boolean.

### Built-in Functions

| Function | Description |
|----------|-------------|
| `variable(name)` | Get variable value |
| `env(name)` | Get environment variable |
| `vision_detect(image, classes)` | Run detection, return first match |
| `vision_ocr(region)` | Run OCR, return text |
| `window_exists(title)` | Check if window exists |
| `window_active(title)` | Check if window is focused |
| `file_exists(path)` | Check file existence |
| `time_ms()` | Current timestamp ms |
| `random_int(min, max)` | Random integer |
| `random_float()` | Random float 0-1 |
| `len(list)` | List length |
| `contains(list, item)` | Check membership |

### Operators
- Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
- Logic: `and`, `or`, `not`
- Arithmetic: `+`, `-`, `*`, `/`, `%`
- String: `in`, `+`

### Examples
```yaml
condition: "variable('health') > 50 and variable('ammo') > 0"
condition: "'error' not in vision_ocr({x:0, y:0, w:100, h:50})"
condition: "window_active('Chrome') and time_ms() > variable('start_time') + 5000"
```

---

## Loops

### Loop Variables
Inside loops, these variables are available:
- `loop.index` - Current iteration (0-based)
- `loop.count` - Total iterations (for count loops)
- `loop.item` - Current item (for for-each)
- `loop.first` - True on first iteration
- `loop.last` - True on last iteration

### Loop Control
```yaml
- type: "loop"
  loop:
    type: "count"
    count: 10
  steps:
    - type: "conditional"
      condition: "loop.index == 5"
      steps:
        - type: "log"
          message: "Halfway there!"
    - type: "action"
      action: "delay"
      parameters: { ms: 100 }
```

---

## Macros

### Definition
```yaml
macros:
  macro_name:
    description: "What this macro does"
    parameters:
      param1:
        type: "string"
        default: "default"
        required: false
    steps:
      - type: "action" ...
```

### Invocation
```yaml
- type: "macro"
  macro: "macro_name"
  parameters:
    param1: "custom_value"
```

### Built-in Macros
The system provides built-in macros:
- `launch_app` - Launch application by name
- `close_app` - Close application gracefully
- `type_text` - Type text with human timing
- `click_at` - Click at coordinates
- `wait_for_window` - Wait for window to appear
- `take_screenshot` - Capture screen region
- `save_file` - Save current document (Ctrl+S)
- `copy_paste` - Copy and paste selection

---

## Import/Include

```yaml
# Import common macros
import:
  - "macros/common.yaml"
  - "macros/photoshop.yaml"

# Or inline include
include: "subtasks/login.yaml"
```

---

## Complete Example: Web Automation

```yaml
name: "GitHub Star Repository"
version: "1.0"
description: "Search GitHub and star a repository"

variables:
  repo_name:
    type: "string"
    default: "human-control-system"
    required: true
  github_user:
    type: "string"
    default: ""
    description: "GitHub username (optional)"

macros:
  github_login:
    description: "Login to GitHub if needed"
    steps:
      - type: "wait"
        condition: "not vision_detect('github_login_btn')"
        timeout_ms: 5000
      - type: "conditional"
        condition: "vision_detect('github_login_btn') != null"
        steps:
          - type: "vision"
            action: "detect"
            parameters:
              classes: ["github_login_btn"]
              confidence: 0.8
            output: "login_btn"
          - type: "action"
            action: "mouse"
            parameters:
              x: "{{login_btn.bbox.x + login_btn.bbox.width/2}}"
              y: "{{login_btn.bbox.y + login_btn.bbox.height/2}}"
              button: "left"
              absolute: true
          - type: "wait"
            condition: "vision_detect('github_username_field') != null"
            timeout_ms: 5000

steps:
  - id: "open_github"
    type: "adapter"
    service: "chrome"
    action: "navigate"
    parameters:
      url: "https://github.com"
      wait_until: "networkidle"

  - id: "login"
    type: "macro"
    macro: "github_login"

  - id: "search_repo"
    type: "vision"
    action: "detect"
    parameters:
      classes: ["search_input"]
      confidence: 0.7
    output: "search_box"

  - id: "type_search"
    type: "action"
    action: "mouse"
    parameters:
      x: "{{search_box.bbox.x + 10}}"
      y: "{{search_box.bbox.y + search_box.bbox.height/2}}"
      button: "left"
      absolute: true
    - type: "action"
      action: "type"
      parameters:
        text: "{{repo_name}}"
        wpm: 80
    - type: "action"
      action: "key"
      parameters: { key: "Enter", state: "tap" }

  - id: "wait_results"
    type: "wait"
    condition: "vision_detect('repo_result') != null"
    timeout_ms: 10000

  - id: "click_repo"
    type: "vision"
    action: "detect"
    parameters:
      classes: ["repo_result"]
      confidence: 0.7
    output: "repo_link"

  - id: "navigate_repo"
    type: "action"
    action: "mouse"
    parameters:
      x: "{{repo_link.bbox.x + repo_link.bbox.width/2}}"
      y: "{{repo_link.bbox.y + repo_link.bbox.height/2}}"
      button: "left"
      absolute: true

  - id: "find_star"
    type: "wait"
    condition: "vision_detect('star_button') != null"
    timeout_ms: 5000

  - id: "click_star"
    type: "vision"
    action: "detect"
    parameters:
      classes: ["star_button"]
      confidence: 0.8
    output: "star_btn"

  - id: "star_repo"
    type: "action"
    action: "mouse"
    parameters:
      x: "{{star_btn.bbox.x + star_btn.bbox.width/2}}"
      y: "{{star_btn.bbox.y + star_btn.bbox.height/2}}"
      button: "left"
      absolute: true

  - id: "verify_starred"
    type: "wait"
    condition: "vision_detect('starred_button') != null"
    timeout_ms: 3000

  - id: "success"
    type: "log"
    level: "info"
    message: "Successfully starred {{repo_name}}!"
```

---

## CLI Usage

```bash
# Run task
hcs run task.yaml

# Run with variable overrides
hcs run task.yaml --var repo_name=my-repo --var github_user=me

# Dry run (validate only)
hcs run task.yaml --dry-run

# Save recording
hcs run task.yaml --record output.rec

# REPL for interactive development
hcs repl

# In REPL:
# > load task.yaml
# > run
# > step
# > vars
# > help
```

---

## Best Practices

1. **Use descriptive IDs** - Makes debugging easier
2. **Keep steps small** - Easier to debug and retry
3. **Use conditions** - Make tasks resilient to UI changes
4. **Set timeouts** - Prevent hanging on missing elements
5. **Use variables** - Makes tasks reusable and configurable
6. **Add logging** - Visibility into execution
7. **Test incrementally** - Use REPL for development
8. **Handle errors** - Use conditional/else branches for fallbacks

---

## Migration from v1

If upgrading from v1 DSL:
- `sequence` → `sequential`
- `parallel` block syntax unchanged
- `if`/`else` → `conditional` with `steps`/`else_steps`
- `for`/`while` → `loop` with `type` field
- Variable syntax: `${var}` → `{{var}}`
- Macro calls: `macro: name` unchanged

---

## Changelog

| Version | Changes |
|---------|---------|
| 1.0.0 | Initial release |
| 1.1.0 | Added vision/brain/adapter steps, inline macros |
| 1.2.0 | Added loop iterators, variable step, wait step |