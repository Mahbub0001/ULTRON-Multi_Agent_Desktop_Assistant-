# Task DSL Examples

This document provides practical examples of Task DSL usage for common automation scenarios.

## Table of Contents
1. [Basic Input Automation](#basic-input-automation)
2. [Application Launch & Control](#application-launch--control)
3. [Computer Vision Integration](#computer-vision-integration)
4. [Web Automation with Chrome](#web-automation-with-chrome)
5. [Photoshop Batch Processing](#photoshop-batch-processing)
6. [Game Automation](#game-automation)
7. [Decision Making with Brain](#decision-making-with-brain)
8. [Recording & Replay](#recording--replay)
9. [Advanced Patterns](#advanced-patterns)

---

## Basic Input Automation

### Simple Key Sequence
```yaml
name: "Copy-Paste Shortcut"
version: "1.0"
steps:
  - id: "select_all"
    type: "action"
    action: "shortcut"
    parameters:
      keys: ["ctrl", "a"]
  
  - id: "copy"
    type: "action"
    action: "shortcut"
    parameters:
      keys: ["ctrl", "c"]
  
  - id: "delay"
    type: "action"
    action: "delay"
    parameters:
      ms: 100
  
  - id: "paste"
    type: "action"
    action: "shortcut"
    parameters:
      keys: ["ctrl", "v"]
```

### Typing with Human Timing
```yaml
name: "Type Document"
version: "1.0"
variables:
  text:
    type: "string"
    default: "Hello, World!"
  wpm:
    type: "int"
    default: 60
steps:
  - id: "type_text"
    type: "action"
    action: "type"
    parameters:
      text: "{{text}}"
      wpm: "{{wpm}}"
      variance: 0.15  # ±15% timing variation
```

### Mouse Drawing
```yaml
name: "Draw Square"
version: "1.0"
variables:
  start_x:
    type: "int"
    default: 500
  start_y:
    type: "int"
    default: 500
  size:
    type: "int"
    default: 200
steps:
  - id: "move_to_start"
    type: "action"
    action: "mouse"
    parameters:
      x: "{{start_x}}"
      y: "{{start_y}}"
      absolute: true
  
  - id: "mouse_down"
    type: "action"
    action: "mouse"
    parameters:
      button: "left"
      button_state: "down"
  
  - id: "draw_right"
    type: "action"
    action: "mouse"
    parameters:
      dx: "{{size}}"
      dy: 0
      absolute: false
  
  - id: "draw_down"
    type: "action"
    action: "mouse"
    parameters:
      dx: 0
      dy: "{{size}}"
  
  - id: "draw_left"
    type: "action"
    action: "mouse"
    parameters:
      dx: "-{{size}}"
      dy: 0
  
  - id: "draw_up"
    type: "action"
    action: "mouse"
    parameters:
      dx: 0
      dy: "-{{size}}"
  
  - id: "mouse_up"
    type: "action"
    action: "mouse"
    parameters:
      button: "left"
      button_state: "up"
```

---

## Application Launch & Control

### Launch and Wait
```yaml
name: "Launch Notepad"
version: "1.0"
macros:
  launch_notepad:
    steps:
      - type: "action"
        action: "shortcut"
        parameters:
          keys: ["win", "r"]
      - type: "action"
        action: "delay"
        parameters: { ms: 500 }
      - type: "action"
        action: "type"
        parameters:
          text: "notepad"
          wpm: 120
      - type: "action"
        action: "key"
        parameters: { key: "Enter", state: "tap" }
      - type: "wait"
        condition: "window_exists('Untitled - Notepad')"
        timeout_ms: 5000

steps:
  - id: "launch"
    type: "macro"
    macro: "launch_notepad"
  
  - id: "verify"
    type: "conditional"
    condition: "window_active('Untitled - Notepad')"
    steps:
      - type: "log"
        message: "Notepad ready"
    else_steps:
      - type: "log"
        level: "error"
        message: "Notepad failed to launch"
```

### Window Management
```yaml
name: "Arrange Windows"
version: "1.0"
variables:
  apps:
    type: "list"
    default: ["notepad", "chrome", "terminal"]
steps:
  - id: "get_monitors"
    type: "adapter"
    service: "window"
    action: "get_monitors"
    output: "monitors"

  - id: "arrange_grid"
    type: "loop"
    loop:
      type: "for_each"
      collection: "{{apps}}"
      iterator: "app"
    steps:
      - id: "find_window"
        type: "adapter"
        service: "window"
        action: "find_window"
        parameters:
          title_filter: "{{app}}"
        output: "window"

      - id: "calculate_position"
        type: "variable"
        action: "set"
        name: "pos"
        value: |
          {
            "x": (loop.index % 2) * (monitors[0].width / 2),
            "y": (loop.index // 2) * (monitors[0].height / 2),
            "width": monitors[0].width / 2,
            "height": monitors[0].height / 2
          }

      - id: "move_resize"
        type: "adapter"
        service: "window"
        action: "move_resize"
        parameters:
          handle: "{{window.window.handle}}"
          x: "{{pos.x}}"
          y: "{{pos.y}}"
          width: "{{pos.width}}"
          height: "{{pos.height}}"
```

---

## Computer Vision Integration

### Find and Click Image
```yaml
name: "Click Button by Image"
version: "1.0"
variables:
  button_image:
    type: "string"
    default: "assets/submit_button.png"
  confidence:
    type: "float"
    default: 0.8
steps:
  - id: "capture_screen"
    type: "vision"
    action: "capture"
    parameters:
      monitor: 0
    output: "screen"

  - id: "find_button"
    type: "vision"
    action: "detect"
    parameters:
      model: "template_matching"  # Custom template model
      template: "{{button_image}}"
      confidence: "{{confidence}}"
      region: "{{screen}}"
    output: "detection"

  - id: "click_if_found"
    type: "conditional"
    condition: "detection != null and len(detection) > 0"
    steps:
      - id: "calculate_center"
        type: "variable"
        action: "set"
        name: "click_pos"
        value: |
          {
            "x": detection[0].bbox.x + detection[0].bbox.width / 2,
            "y": detection[0].bbox.y + detection[0].bbox.height / 2
          }
      - id: "click"
        type: "action"
        action: "mouse"
        parameters:
          x: "{{click_pos.x}}"
          y: "{{click_pos.y}}"
          button: "left"
          absolute: true
    else_steps:
      - id: "not_found"
        type: "log"
        level: "warn"
        message: "Button not found on screen"
```

### OCR Text Extraction
```yaml
name: "Read Screen Text"
version: "1.0"
variables:
  region:
    type: "map"
    default: { x: 100, y: 100, width: 500, height: 200 }
steps:
  - id: "capture_region"
    type: "vision"
    action: "capture"
    parameters:
      region: "{{region}}"
    output: "image"

  - id: "ocr"
    type: "vision"
    action: "ocr"
    parameters:
      image: "{{image}}"
      languages: ["en"]
      engine: "paddle"
    output: "text_blocks"

  - id: "extract_text"
    type: "variable"
    action: "set"
    name: "full_text"
    value: |
      " ".join([block.text for block in text_blocks])

  - id: "log_result"
    type: "log"
    message: "Extracted text: {{full_text}}"

  - id: "check_keyword"
    type: "conditional"
    condition: "'error' in full_text.lower()"
    steps:
      - type: "log"
        level: "error"
        message: "Error detected in text!"
    else_steps:
      - type: "log"
        message: "No errors found"
```

### Object Detection Loop
```yaml
name: "Monitor for Objects"
version: "1.0"
variables:
  target_classes:
    type: "list"
    default: ["person", "cell phone", "laptop"]
  check_interval_ms:
    type: "int"
    default: 1000
  max_checks:
    type: "int"
    default: 60
steps:
  - id: "monitor_loop"
    type: "loop"
    loop:
      type: "count"
      count: "{{max_checks}}"
    steps:
      - id: "capture"
        type: "vision"
        action: "capture"
        output: "frame"

      - id: "detect"
        type: "vision"
        action: "detect"
        parameters:
          image: "{{frame}}"
          classes: "{{target_classes}}"
          confidence: 0.6
        output: "detections"

      - id: "process_detections"
        type: "conditional"
        condition: "len(detections) > 0"
        steps:
          - id: "log_detections"
            type: "loop"
            loop:
              type: "for_each"
              collection: "{{detections}}"
              iterator: "det"
            steps:
              - type: "log"
                message: "Detected {{det.class_name}} at ({{det.bbox.x}}, {{det.bbox.y}}) conf={{det.confidence}}"
              - type: "conditional"
                condition: "det.class_name == 'person' and det.confidence > 0.8"
                steps:
                  - type: "adapter"
                    service: "window"
                    action: "capture_window"
                    parameters:
                      handle: "{{active_window}}"
                    output: "evidence"
                  - type: "log"
                    level: "warn"
                    message: "Person detected! Evidence captured."
        else_steps:
          - type: "log"
            level: "debug"
            message: "No targets detected"

      - id: "wait_interval"
        type: "action"
        action: "delay"
        parameters:
          ms: "{{check_interval_ms}}"
```

---

## Web Automation with Chrome

### Login Flow
```yaml
name: "Website Login"
version: "1.0"
variables:
  url:
    type: "string"
    default: "https://example.com/login"
  username:
    type: "string"
    required: true
  password:
    type: "string"
    required: true
steps:
  - id: "connect_chrome"
    type: "adapter"
    service: "chrome"
    action: "connect"
    parameters:
      endpoint: "http://localhost:9222"
    output: "session"

  - id: "navigate"
    type: "adapter"
    service: "chrome"
    action: "navigate"
    parameters:
      session_id: "{{session.session_id}}"
      url: "{{url}}"
      options:
        wait_until_networkidle: true

  - id: "find_username"
    type: "adapter"
    service: "chrome"
    action: "query_selector"
    parameters:
      session_id: "{{session.session_id}}"
      target_id: "{{session.target_id}}"
      selector: "input[name='username'], input[type='email']"
    output: "user_field"

  - id: "type_username"
    type: "adapter"
    service: "chrome"
    action: "type_text"
    parameters:
      session_id: "{{session.session_id}}"
      target_id: "{{session.target_id}}"
      node_id: "{{user_field.node.node_id}}"
      text: "{{username}}"

  - id: "find_password"
    type: "adapter"
    service: "chrome"
    action: "query_selector"
    parameters:
      session_id: "{{session.session_id}}"
      target_id: "{{session.target_id}}"
      selector: "input[name='password'], input[type='password']"
    output: "pass_field"

  - id: "type_password"
    type: "adapter"
    service: "chrome"
    action: "type_text"
    parameters:
      session_id: "{{session.session_id}}"
      target_id: "{{session.target_id}}"
      node_id: "{{pass_field.node.node_id}}"
      text: "{{password}}"

  - id: "submit"
    type: "adapter"
    service: "chrome"
    action: "query_selector"
    parameters:
      session_id: "{{session.session_id}}"
      target_id: "{{session.target_id}}"
      selector: "button[type='submit'], input[type='submit']"
    output: "submit_btn"

  - id: "click_submit"
    type: "adapter"
    service: "chrome"
    action: "click_element"
    parameters:
      session_id: "{{session.session_id}}"
      target_id: "{{session.target_id}}"
      node_id: "{{submit_btn.node.node_id}}"

  - id: "verify_login"
    type: "wait"
    condition: "not vision_detect('login_form')"
    timeout_ms: 10000
```

### Scrape Data
```yaml
name: "Scrape Product Prices"
version: "1.0"
variables:
  urls:
    type: "list"
    required: true
  selector:
    type: "string"
    default: ".product-price"
steps:
  - id: "connect"
    type: "adapter"
    service: "chrome"
    action: "connect"
    parameters:
      endpoint: "http://localhost:9222"
    output: "session"

  - id: "scrape_loop"
    type: "loop"
    loop:
      type: "for_each"
      collection: "{{urls}}"
      iterator: "url"
    steps:
      - id: "navigate"
        type: "adapter"
        service: "chrome"
        action: "navigate"
        parameters:
          session_id: "{{session.session_id}}"
          url: "{{url}}"
          options:
            wait_until_networkidle: true

      - id: "get_prices"
        type: "adapter"
        service: "chrome"
        action: "query_selector_all"
        parameters:
          session_id: "{{session.session_id}}"
          target_id: "{{session.target_id}}"
          selector: "{{selector}}"
        output: "price_elements"

      - id: "extract_prices"
        type: "variable"
        action: "set"
        name: "prices"
        value: |
          [elem.text for elem in price_elements.nodes]

      - id: "log_prices"
        type: "log"
        message: "{{url}}: {{prices}}"

      - id: "save_screenshot"
        type: "adapter"
        service: "chrome"
        action: "capture_screenshot"
        parameters:
          session_id: "{{session.session_id}}"
          target_id: "{{session.target_id}}"
        output: "screenshot"
```

---

## Photoshop Batch Processing

### Apply Action to Folder
```yaml
name: "Batch Resize Images"
version: "1.0"
variables:
  source_folder:
    type: "string"
    required: true
  dest_folder:
    type: "string"
    required: true
  action_name:
    type: "string"
    default: "Resize for Web"
  action_set:
    type: "string"
    default: "My Actions"
steps:
  - id: "batch_process"
    type: "adapter"
    service: "photoshop"
    action: "batch_process"
    parameters:
      action_name: "{{action_name}}"
      action_set: "{{action_set}}"
      source_files: "{{source_folder}}/*"
      destination_folder: "{{dest_folder}}"
      options:
        override_open: true
        include_subfolders: true
        suppress_warnings: true
        file_naming: "Document Name + 2 Digit Serial"
    output: "result"

  - id: "log_result"
    type: "log"
    message: "Processed {{result.processed}} files, {{result.failed}} failed"
    fields:
      processed: "{{result.processed}}"
      failed: "{{result.failed}}"
      errors: "{{result.errors}}"
```

### Layer Manipulation
```yaml
name: "Create Composition"
version: "1.0"
variables:
  bg_image:
    type: "string"
    required: true
  fg_image:
    type: "string"
    required: true
  output_path:
    type: "string"
    required: true
steps:
  - id: "create_doc"
    type: "adapter"
    service: "photoshop"
    action: "create_document"
    parameters:
      width: 1920
      height: 1080
      resolution: 72
      color_mode: "RGB"
      name: "Composition"
    output: "doc"

  - id: "import_bg"
    type: "adapter"
    service: "photoshop"
    action: "import_file"
    parameters:
      path: "{{bg_image}}"
      document_id: "{{doc.document.id}}"
      options:
        place_as_smart_object: true
    output: "bg_layer"

  - id: "import_fg"
    type: "adapter"
    service: "photoshop"
    action: "import_file"
    parameters:
      path: "{{fg_image}}"
      document_id: "{{doc.document.id}}"
      options:
        place_as_smart_object: true
    output: "fg_layer"

  - id: "position_fg"
    type: "adapter"
    service: "photoshop"
    action: "move_layer"
    parameters:
      document_id: "{{doc.document.id}}"
      layer_id: "{{fg_layer.layer_id}}"
      new_index: 0  # Top layer

  - id: "add_mask"
    type: "adapter"
    service: "photoshop"
    action: "apply_layer_style"
    parameters:
      document_id: "{{doc.document.id}}"
      layer_id: "{{fg_layer.layer_id}}"
      style:
        drop_shadow: true
        drop_shadow_params:
          opacity: 0.5
          angle: 120
          distance: 10
          size: 15

  - id: "export"
    type: "adapter"
    service: "photoshop"
    action: "export_document"
    parameters:
      document_id: "{{doc.document.id}}"
      path: "{{output_path}}"
      format: "PNG"
      options:
        quality: 100
        transparency: true
```

---

## Game Automation

### Health Monitor
```yaml
name: "Auto Heal"
version: "1.0"
variables:
  game_process:
    type: "string"
    default: "game.exe"
  health_address:
    type: "string"
    default: "0x140ABCDEF"
  heal_key:
    type: "string"
    default: "F1"
  health_threshold:
    type: "int"
    default: 50
steps:
  - id: "attach_game"
    type: "adapter"
    service: "game"
    action: "attach_process"
    parameters:
      name: "{{game_process}}"
      access: "READ_WRITE"
    output: "process"

  - id: "monitor_loop"
    type: "loop"
    loop:
      type: "while"
      condition: "true"
    steps:
      - id: "read_health"
        type: "adapter"
        service: "game"
        action: "read_memory"
        parameters:
          pid: "{{process.process.pid}}"
          address: "{{health_address}}"
          size: 4
        output: "health_data"

      - id: "parse_health"
        type: "variable"
        action: "set"
        name: "current_health"
        value: |
          int.from_bytes(health_data.data, 'little')

      - id: "check_health"
        type: "conditional"
        condition: "current_health < health_threshold"
        steps:
          - id: "heal"
            type: "action"
            action: "key"
            parameters:
              key: "{{heal_key}}"
              state: "tap"
          - id: "log_heal"
            type: "log"
            level: "info"
            message: "Healed! Health was {{current_health}}"

      - id: "wait"
        type: "action"
        action: "delay"
        parameters:
          ms: 100
```

### Pattern Scan for Addresses
```yaml
name: "Find Game Addresses"
version: "1.0"
variables:
  game_process:
    type: "string"
    default: "game.exe"
  patterns:
    type: "map"
    default:
      health: "48 8B 05 ?? ?? ?? ?? 48 85 C0 74 ?? 8B 40 ??"
      ammo: "48 8B 0D ?? ?? ?? ?? 8B 41 ?? 85 C0"
steps:
  - id: "attach"
    type: "adapter"
    service: "game"
    action: "attach_process"
    parameters:
      name: "{{game_process}}"
    output: "process"

  - id: "scan_patterns"
    type: "loop"
    loop:
      type: "for_each"
      collection: "{{patterns}}"
      iterator: "pattern_entry"
    steps:
      - id: "scan"
        type: "adapter"
        service: "game"
        action: "scan_pattern"
        parameters:
          pid: "{{process.process.pid}}"
          pattern: "{{pattern_entry.value}}"
          options:
            executable: true
            max_results: 5
        output: "results"

      - id: "store_address"
        type: "conditional"
        condition: "len(results.addresses) > 0"
        steps:
          - type: "variable"
            action: "set"
            name: "addresses.{{pattern_entry.key}}"
            value: "{{results.addresses[0]}}"
          - type: "log"
            message: "Found {{pattern_entry.key}} at {{results.addresses[0]}}"
        else_steps:
          - type: "log"
            level: "error"
            message: "Pattern {{pattern_entry.key}} not found!"
```

---

## Decision Making with Brain

### Combat AI Loop
```yaml
name: "Combat Bot"
version: "1.0"
variables:
  tree_id:
    type: "string"
    default: "combat_ai"
steps:
  - id: "load_tree"
    type: "brain"
    action: "load"
    parameters:
      tree_id: "{{tree_id}}"
      tree_definition: |
        {
          "root": {
            "type": "Selector",
            "children": [
              {
                "type": "Sequence",
                "name": "Emergency Heal",
                "children": [
                  { "type": "Condition", "name": "Critical Health", "plugin": "status", "function": "check_health", "args": { "threshold": 20 } },
                  { "type": "Action", "name": "Use Potion", "plugin": "input", "function": "press_key", "args": { "key": "F1" } }
                ]
              },
              {
                "type": "Sequence",
                "name": "Combat",
                "children": [
                  { "type": "Condition", "name": "Target Visible", "plugin": "vision", "function": "detect_enemy" },
                  { "type": "Action", "name": "Attack", "plugin": "combat", "function": "attack_target" }
                ]
              },
              {
                "type": "Action",
                "name": "Patrol",
                "plugin": "movement",
                "function": "patrol"
              }
            ]
          }
        }
      initial_blackboard:
        health: 100
        mana: 100

  - id: "combat_loop"
    type: "loop"
    loop:
      type: "while"
      condition: "true"
    steps:
      - id: "tick_brain"
        type: "brain"
        action: "tick"
        parameters:
          tree_id: "{{tree_id}}"
          blackboard_updates:
            health: "{{get_health()}}"
            mana: "{{get_mana()}}"
            target_visible: "{{vision_detect('enemy') != null}}"
        output: "tick_result"

      - id: "log_status"
        type: "conditional"
        condition: "tick_result.status != 'RUNNING'"
        steps:
          - type: "log"
            message: "Tree status: {{tick_result.status}}"
```

---

## Recording & Replay

### Record User Actions
```yaml
name: "Record Session"
version: "1.0"
variables:
  duration_seconds:
    type: "int"
    default: 60
  output_file:
    type: "string"
    default: "my_recording.rec"
steps:
  - id: "start_recording"
    type: "adapter"
    service: "orchestrator"
    action: "start_recording"
    parameters:
      name: "user_session_{{time_ms()}}"
      capture_keyboard: true
      capture_mouse: true
      capture_delays: true
      max_duration_seconds: "{{duration_seconds}}"

  - id: "wait_recording"
    type: "action"
    action: "delay"
    parameters:
      ms: "{{duration_seconds * 1000}}"

  - id: "stop_recording"
    type: "adapter"
    service: "orchestrator"
    action: "stop_recording"
    output: "recording"

  - id: "save_recording"
    type: "adapter"
    service: "orchestrator"
    action: "get_recording"
    parameters:
      recording_id: "{{recording.id}}"
    output: "full_recording"
```

### Replay with Variations
```yaml
name: "Replay with Variations"
version: "1.0"
variables:
  recording_id:
    type: "string"
    required: true
  speed:
    type: "float"
    default: 1.0
  loop_count:
    type: "int"
    default: 1
steps:
  - id: "replay_loop"
    type: "loop"
    loop:
      type: "count"
      count: "{{loop_count}}"
    steps:
      - id: "replay"
        type: "adapter"
        service: "orchestrator"
        action: "replay_recording"
        parameters:
          recording_id: "{{recording_id}}"
          speed: "{{speed}}"
          loop_playback: false
          variable_overrides:
            replay_iteration: "{{loop.index}}"
        output: "result"

      - id: "log_replay"
        type: "log"
        message: "Replay {{loop.index + 1}}/{{loop_count}} completed: {{result.success}}"
```

---

## Advanced Patterns

### Error Recovery Pattern
```yaml
name: "Robust Automation"
version: "1.0"
macros:
  try_action:
    description: "Try action with retries and fallback"
    parameters:
      action:
        type: "macro"
        required: true
      fallback:
        type: "macro"
        required: false
      max_retries:
        type: "int"
        default: 3
    steps:
      - id: "attempt_loop"
        type: "loop"
        loop:
          type: "count"
          count: "{{max_retries}}"
        steps:
          - id: "try"
            type: "macro"
            macro: "{{action}}"
          - id: "check_success"
            type: "conditional"
            condition: "variable('last_action_success') == true"
            steps:
              - type: "variable"
                action: "set"
                name: "action_succeeded"
                value: true
              - type: "break"  # Exit loop
            else_steps:
              - type: "log"
                level: "warn"
                message: "Attempt {{loop.index + 1}} failed, retrying..."
              - type: "action"
                action: "delay"
                parameters: { ms: 1000 }

      - id: "handle_failure"
        type: "conditional"
        condition: "variable('action_succeeded') != true"
        steps:
          - type: "log"
            level: "error"
            message: "All retries exhausted"
          - type: "conditional"
            condition: "fallback != ''"
            steps:
              - type: "macro"
                macro: "{{fallback}}"

steps:
  - id: "main_task"
    type: "macro"
    macro: "try_action"
    parameters:
      action: "click_submit_button"
      fallback: "submit_via_javascript"
      max_retries: 3
```

### Parallel Data Processing
```yaml
name: "Parallel Processing"
version: "1.0"
variables:
  items:
    type: "list"
    required: true
  max_parallel:
    type: "int"
    default: 4
steps:
  - id: "chunk_items"
    type: "variable"
    action: "set"
    name: "chunks"
    value: |
      [items[i:i+max_parallel] for i in range(0, len(items), max_parallel)]

  - id: "process_chunks"
    type: "loop"
    loop:
      type: "for_each"
      collection: "{{chunks}}"
      iterator: "chunk"
    steps:
      - id: "parallel_process"
        type: "parallel"
        wait_for: "all"
        steps:
          - type: "loop"
            loop:
              type: "for_each"
              collection: "{{chunk}}"
              iterator: "item"
            steps:
              - id: "process_item"
                type: "macro"
                macro: "process_single_item"
                parameters:
                  item: "{{item}}"
```

### State Machine Pattern
```yaml
name: "State Machine Task"
version: "1.0"
variables:
  state:
    type: "string"
    default: "INIT"
  max_transitions:
    type: "int"
    default: 100
steps:
  - id: "state_loop"
    type: "loop"
    loop:
      type: "while"
      condition: "state != 'DONE' and state != 'ERROR'"
      max_iterations: "{{max_transitions}}"
    steps:
      - id: "state_INIT"
        type: "conditional"
        condition: "state == 'INIT'"
        steps:
          - type: "macro"
            macro: "initialize"
          - type: "variable"
            action: "set"
            name: "state"
            value: "READY"

      - id: "state_READY"
        type: "conditional"
        condition: "state == 'READY'"
        steps:
          - type: "wait"
            condition: "vision_detect('start_button') != null"
            timeout_ms: 30000
          - type: "variable"
            action: "set"
            name: "state"
            value: "RUNNING"

      - id: "state_RUNNING"
        type: "conditional"
        condition: "state == 'RUNNING'"
        steps:
          - type: "macro"
            macro: "execute_step"
          - type: "conditional"
            condition: "variable('step_complete') == true"
            steps:
              - type: "variable"
                action: "set"
                name: "state"
                value: "READY"
            else_steps:
              - type: "variable"
                action: "set"
                name: "state"
                value: "DONE"

      - id: "state_ERROR"
        type: "conditional"
        condition: "state == 'ERROR'"
        steps:
          - type: "macro"
            macro: "cleanup"
          - type: "variable"
            action: "set"
            name: "state"
            value: "DONE"
```

---

## Running Examples

```bash
# Run any example
hcs run examples/basic_input.yaml

# With custom variables
hcs run examples/web_login.yaml --var username=myuser --var password=secret

# Dry run to validate
hcs run examples/photoshop_batch.yaml --dry-run

# Record execution
hcs run examples/game_heal.yaml --record heal_session.rec

# REPL for interactive testing
hcs repl
> load examples/vision_click.yaml
> step
> vars
> continue
```

---

## Example Files Location

All examples are available in the repository:
```
human-control-system/
├── examples/
│   ├── basic_input.yaml
│   ├── app_launch.yaml
│   ├── vision_click.yaml
│   ├── web_login.yaml
│   ├── photoshop_batch.yaml
│   ├── game_heal.yaml
│   ├── brain_combat.yaml
│   ├── record_replay.yaml
│   └── advanced_patterns.yaml
├── macros/
│   ├── common.yaml
│   ├── photoshop.yaml
│   ├── chrome.yaml
│   └── game.yaml
└── tasks/
    └── ...
```

---

## Contributing Examples

To add a new example:
1. Create `.yaml` file in `examples/`
2. Follow naming convention: `category_description.yaml`
3. Include all required variables with defaults
4. Add comments explaining the workflow
5. Test with `hcs run --dry-run`
6. Submit PR with description of use case