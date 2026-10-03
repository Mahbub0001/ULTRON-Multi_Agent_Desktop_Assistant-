use starlark::environment::{GlobalsBuilder, Module};
use starlark::values::{Value, Heap, List, Dict};
use starlark::eval::Evaluator;
use anyhow::Result;

pub fn create_stdlib() -> Module {
    let mut globals = GlobalsBuilder::standard();
    
    // Time functions
    globals.set("sleep", sleep_fn);
    globals.set("now", now_fn);
    globals.set("timestamp", timestamp_fn);
    
    // String functions
    globals.set("str_join", str_join_fn);
    globals.set("str_split", str_split_fn);
    globals.set("str_replace", str_replace_fn);
    globals.set("str_contains", str_contains_fn);
    globals.set("str_startswith", str_startswith_fn);
    globals.set("str_endswith", str_endswith_fn);
    globals.set("str_lower", str_lower_fn);
    globals.set("str_upper", str_upper_fn);
    globals.set("str_trim", str_trim_fn);
    
    // List functions
    globals.set("list_len", list_len_fn);
    globals.set("list_get", list_get_fn);
    globals.set("list_append", list_append_fn);
    globals.set("list_extend", list_extend_fn);
    globals.set("list_pop", list_pop_fn);
    globals.set("list_index", list_index_fn);
    globals.set("list_reverse", list_reverse_fn);
    globals.set("list_sort", list_sort_fn);
    
    // Dict functions
    globals.set("dict_get", dict_get_fn);
    globals.set("dict_set", dict_set_fn);
    globals.set("dict_has", dict_has_fn);
    globals.set("dict_keys", dict_keys_fn);
    globals.set("dict_values", dict_values_fn);
    globals.set("dict_items", dict_items_fn);
    globals.set("dict_remove", dict_remove_fn);
    globals.set("dict_clear", dict_clear_fn);
    globals.set("dict_copy", dict_copy_fn);
    
    // Math functions
    globals.set("math_min", math_min_fn);
    globals.set("math_max", math_max_fn);
    globals.set("math_abs", math_abs_fn);
    globals.set("math_round", math_round_fn);
    globals.set("math_floor", math_floor_fn);
    globals.set("math_ceil", math_ceil_fn);
    globals.set("math_pow", math_pow_fn);
    globals.set("math_sqrt", math_sqrt_fn);
    globals.set("math_random", math_random_fn);
    globals.set("math_random_int", math_random_int_fn);
    
    // JSON functions
    globals.set("json_encode", json_encode_fn);
    globals.set("json_decode", json_decode_fn);
    
    // File functions
    globals.set("file_read", file_read_fn);
    globals.set("file_write", file_write_fn);
    globals.set("file_exists", file_exists_fn);
    globals.set("file_list", file_list_fn);
    globals.set("file_mkdir", file_mkdir_fn);
    globals.set("file_remove", file_remove_fn);
    
    // System functions
    globals.set("env_get", env_get_fn);
    globals.set("env_set", env_set_fn);
    globals.set("exec_command", exec_command_fn);
    globals.set("exec_output", exec_output_fn);
    
    // HCS-specific functions
    globals.set("hcs_key_down", hcs_key_down_fn);
    globals.set("hcs_key_up", hcs_key_up_fn);
    globals.set("hcs_key_press", hcs_key_press_fn);
    globals.set("hcs_mouse_move", hcs_mouse_move_fn);
    globals.set("hcs_mouse_click", hcs_mouse_click_fn);
    globals.set("hcs_mouse_scroll", hcs_mouse_scroll_fn);
    globals.set("hcs_type_text", hcs_type_text_fn);
    globals.set("hcs_delay", hcs_delay_fn);
    globals.set("hcs_capture_screen", hcs_capture_screen_fn);
    globals.set("hcs_detect_objects", hcs_detect_objects_fn);
    globals.set("hcs_recognize_text", hcs_recognize_text_fn);
    globals.set("hcs_execute_macro", hcs_execute_macro_fn);
    globals.set("hcs_log", hcs_log_fn);
    
    globals.build()
}

// Time functions
fn sleep_fn(ms: i64, _heap: &Heap) -> Result<Value, String> {
    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    Ok(Value::new_none())
}

fn now_fn(_heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_string(&chrono::Utc::now().to_rfc3339()))
}

fn timestamp_fn(_heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_int(chrono::Utc::now().timestamp_millis()))
}

// String functions
fn str_join_fn(separator: String, list: List, _heap: &Heap) -> Result<Value, String> {
    let items: Vec<String> = list.iter().map(|v| v.to_string()).collect();
    Ok(Value::new_string(&items.join(&separator)))
}

fn str_split_fn(s: String, separator: String, _heap: &Heap) -> Result<Value, String> {
    let parts: Vec<Value> = s.split(&separator).map(|p| Value::new_string(p)).collect();
    Ok(Value::new_list(parts))
}

fn str_replace_fn(s: String, from: String, to: String, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_string(&s.replace(&from, &to)))
}

fn str_contains_fn(s: String, substr: String, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_bool(s.contains(&substr)))
}

fn str_startswith_fn(s: String, prefix: String, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_bool(s.starts_with(&prefix)))
}

fn str_endswith_fn(s: String, suffix: String, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_bool(s.ends_with(&suffix)))
}

fn str_lower_fn(s: String, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_string(&s.to_lowercase()))
}

fn str_upper_fn(s: String, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_string(&s.to_uppercase()))
}

fn str_trim_fn(s: String, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_string(&s.trim()))
}

// List functions
fn list_len_fn(list: List, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_int(list.length() as i64))
}

fn list_get_fn(list: List, index: i64, _heap: &Heap) -> Result<Value, String> {
    let len = list.length() as i64;
    let idx = if index < 0 { len + index } else { index };
    if idx < 0 || idx >= len {
        return Err("Index out of bounds".to_string());
    }
    Ok(list.get(idx as usize).unwrap())
}

fn list_append_fn(mut list: List, value: Value, _heap: &Heap) -> Result<Value, String> {
    list.push(value)?;
    Ok(Value::new_none())
}

fn list_extend_fn(mut list: List, other: List, _heap: &Heap) -> Result<Value, String> {
    for item in other.iter() {
        list.push(item)?;
    }
    Ok(Value::new_none())
}

fn list_pop_fn(mut list: List, index: Option<i64>, _heap: &Heap) -> Result<Value, String> {
    let len = list.length() as i64;
    let idx = match index {
        Some(i) => if i < 0 { len + i } else { i },
        None => len - 1,
    };
    if idx < 0 || idx >= len {
        return Err("Index out of bounds".to_string());
    }
    // Starlark List doesn't have pop, so we'd need to rebuild
    Err("list_pop not implemented".to_string())
}

fn list_index_fn(list: List, value: Value, _heap: &Heap) -> Result<Value, String> {
    for (i, item) in list.iter().enumerate() {
        if item.equals(value)? {
            return Ok(Value::new_int(i as i64));
        }
    }
    Err("Value not found".to_string())
}

fn list_reverse_fn(mut list: List, _heap: &Heap) -> Result<Value, String> {
    // Would need to rebuild list
    Err("list_reverse not implemented".to_string())
}

fn list_sort_fn(mut list: List, _heap: &Heap) -> Result<Value, String> {
    Err("list_sort not implemented".to_string())
}

// Dict functions
fn dict_get_fn(dict: Dict, key: Value, default: Option<Value>, _heap: &Heap) -> Result<Value, String> {
    match dict.get(key) {
        Some(v) => Ok(v),
        None => Ok(default.unwrap_or(Value::new_none())),
    }
}

fn dict_set_fn(mut dict: Dict, key: Value, value: Value, _heap: &Heap) -> Result<Value, String> {
    dict.insert(key, value)?;
    Ok(Value::new_none())
}

fn dict_has_fn(dict: Dict, key: Value, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_bool(dict.get(key).is_some()))
}

fn dict_keys_fn(dict: Dict, _heap: &Heap) -> Result<Value, String> {
    let keys: Vec<Value> = dict.iter().map(|(k, _)| k).collect();
    Ok(Value::new_list(keys))
}

fn dict_values_fn(dict: Dict, _heap: &Heap) -> Result<Value, String> {
    let values: Vec<Value> = dict.iter().map(|(_, v)| v).collect();
    Ok(Value::new_list(values))
}

fn dict_items_fn(dict: Dict, _heap: &Heap) -> Result<Value, String> {
    let items: Vec<Value> = dict.iter().map(|(k, v)| {
        Value::new_tuple(vec![k, v])
    }).collect();
    Ok(Value::new_list(items))
}

fn dict_remove_fn(mut dict: Dict, key: Value, _heap: &Heap) -> Result<Value, String> {
    dict.remove(key);
    Ok(Value::new_none())
}

fn dict_clear_fn(mut dict: Dict, _heap: &Heap) -> Result<Value, String> {
    dict.clear();
    Ok(Value::new_none())
}

fn dict_copy_fn(dict: Dict, _heap: &Heap) -> Result<Value, String> {
    let new_dict = Dict::new(_heap);
    for (k, v) in dict.iter() {
        new_dict.insert(k, v)?;
    }
    Ok(Value::new_dict(new_dict))
}

// Math functions
fn math_min_fn(a: f64, b: f64, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_float(a.min(b)))
}

fn math_max_fn(a: f64, b: f64, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_float(a.max(b)))
}

fn math_abs_fn(x: f64, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_float(x.abs()))
}

fn math_round_fn(x: f64, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_int(x.round() as i64))
}

fn math_floor_fn(x: f64, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_int(x.floor() as i64))
}

fn math_ceil_fn(x: f64, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_int(x.ceil() as i64))
}

fn math_pow_fn(base: f64, exp: f64, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_float(base.powf(exp)))
}

fn math_sqrt_fn(x: f64, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_float(x.sqrt()))
}

fn math_random_fn(_heap: &Heap) -> Result<Value, String> {
    use rand::Rng;
    Ok(Value::new_float(rand::thread_rng().gen::<f64>()))
}

fn math_random_int_fn(min: i64, max: i64, _heap: &Heap) -> Result<Value, String> {
    use rand::Rng;
    Ok(Value::new_int(rand::thread_rng().gen_range(min..=max)))
}

// JSON functions
fn json_encode_fn(value: Value, _heap: &Heap) -> Result<Value, String> {
    let json = serde_json::to_string(&value_to_json(value)?)
        .map_err(|e| e.to_string())?;
    Ok(Value::new_string(&json))
}

fn json_decode_fn(s: String, _heap: &Heap) -> Result<Value, String> {
    let json: serde_json::Value = serde_json::from_str(&s).map_err(|e| e.to_string())?;
    Ok(json_to_value(&json, _heap)?)
}

fn value_to_json(value: Value) -> Result<serde_json::Value, String> {
    Ok(match value.unpack() {
        Some(v) if v.is_none() => serde_json::Value::Null,
        Some(v) if v.is_bool() => serde_json::Value::Bool(v.to_bool()),
        Some(v) if v.is_int() => serde_json::Value::Number(serde_json::Number::from(v.to_int())),
        Some(v) if v.is_float() => serde_json::Value::Number(serde_json::Number::from_f64(v.to_float()).unwrap()),
        Some(v) if v.is_string() => serde_json::Value::String(v.to_string()),
        Some(v) if v.is_list() => {
            let list = v.to_list().unwrap();
            let arr: Vec<serde_json::Value> = list.iter().map(|v| value_to_json(v).unwrap()).collect();
            serde_json::Value::Array(arr)
        }
        Some(v) if v.is_dict() => {
            let dict = v.to_dict().unwrap();
            let mut obj = serde_json::Map::new();
            for (k, v) in dict.iter() {
                obj.insert(k.to_string(), value_to_json(v).unwrap());
            }
            serde_json::Value::Object(obj)
        }
        _ => return Err("Unsupported value type".to_string()),
    })
}

fn json_to_value(json: &serde_json::Value, heap: &Heap) -> Result<Value, String> {
    Ok(match json {
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
        serde_json::Value::String(s) => Value::new_string(s),
        serde_json::Value::Array(arr) => {
            let list: Vec<Value> = arr.iter().map(|v| json_to_value(v, heap).unwrap()).collect();
            Value::new_list(list)
        }
        serde_json::Value::Object(obj) => {
            let dict = Dict::new(heap);
            for (k, v) in obj {
                dict.insert(Value::new_string(k), json_to_value(v, heap)?)?;
            }
            Value::new_dict(dict)
        }
    })
}

// File functions
fn file_read_fn(path: String, _heap: &Heap) -> Result<Value, String> {
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    Ok(Value::new_string(&content))
}

fn file_write_fn(path: String, content: String, _heap: &Heap) -> Result<Value, String> {
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(Value::new_none())
}

fn file_exists_fn(path: String, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_bool(std::path::Path::new(&path).exists()))
}

fn file_list_fn(path: String, _heap: &Heap) -> Result<Value, String> {
    let entries = std::fs::read_dir(&path).map_err(|e| e.to_string())?;
    let files: Vec<Value> = entries.filter_map(|e| e.ok())
        .map(|e| Value::new_string(&e.file_name().to_string_lossy()))
        .collect();
    Ok(Value::new_list(files))
}

fn file_mkdir_fn(path: String, _heap: &Heap) -> Result<Value, String> {
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(Value::new_none())
}

fn file_remove_fn(path: String, _heap: &Heap) -> Result<Value, String> {
    std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    Ok(Value::new_none())
}

// System functions
fn env_get_fn(key: String, _heap: &Heap) -> Result<Value, String> {
    match std::env::var(&key) {
        Ok(val) => Ok(Value::new_string(&val)),
        Err(_) => Ok(Value::new_none()),
    }
}

fn env_set_fn(key: String, value: String, _heap: &Heap) -> Result<Value, String> {
    std::env::set_var(&key, &value);
    Ok(Value::new_none())
}

fn exec_command_fn(command: String, args: List, _heap: &Heap) -> Result<Value, String> {
    let mut cmd = std::process::Command::new(&command);
    for arg in args.iter() {
        cmd.arg(arg.to_string());
    }
    let output = cmd.output().map_err(|e| e.to_string())?;
    Ok(Value::new_bool(output.status.success()))
}

fn exec_output_fn(command: String, args: List, _heap: &Heap) -> Result<Value, String> {
    let mut cmd = std::process::Command::new(&command);
    for arg in args.iter() {
        cmd.arg(arg.to_string());
    }
    let output = cmd.output().map_err(|e| e.to_string())?;
    let dict = Dict::new(_heap);
    dict.insert(Value::new_string("stdout"), Value::new_string(&String::from_utf8_lossy(&output.stdout)))?;
    dict.insert(Value::new_string("stderr"), Value::new_string(&String::from_utf8_lossy(&output.stderr)))?;
    dict.insert(Value::new_string("exit_code"), Value::new_int(output.status.code().unwrap_or(-1) as i64))?;
    Ok(Value::new_dict(dict))
}

// HCS-specific functions (stubs - would call gRPC client)
fn hcs_key_down_fn(code: i32, _heap: &Heap) -> Result<Value, String> {
    // Would call gRPC client
    Ok(Value::new_none())
}

fn hcs_key_up_fn(code: i32, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_none())
}

fn hcs_key_press_fn(code: i32, delay_ms: Option<i32>, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_none())
}

fn hcs_mouse_move_fn(x: i32, y: i32, absolute: Option<bool>, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_none())
}

fn hcs_mouse_click_fn(button: String, x: Option<i32>, y: Option<i32>, count: Option<i32>, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_none())
}

fn hcs_mouse_scroll_fn(dx: i32, dy: i32, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_none())
}

fn hcs_type_text_fn(text: String, delay_ms: Option<i32>, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_none())
}

fn hcs_delay_fn(microseconds: i64, _heap: &Heap) -> Result<Value, String> {
    std::thread::sleep(std::time::Duration::from_micros(microseconds as u64));
    Ok(Value::new_none())
}

fn hcs_capture_screen_fn(monitor_index: Option<i32>, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_dict(Dict::new(_heap)))
}

fn hcs_detect_objects_fn(image: Value, class_filter: Option<List>, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_list(vec![]))
}

fn hcs_recognize_text_fn(image: Value, region: Option<Dict>, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_list(vec![]))
}

fn hcs_execute_macro_fn(name: String, params: Option<Dict>, _heap: &Heap) -> Result<Value, String> {
    Ok(Value::new_none())
}

fn hcs_log_fn(level: String, message: String, _heap: &Heap) -> Result<Value, String> {
    match level.as_str() {
        "trace" => tracing::trace!("{}", message),
        "debug" => tracing::debug!("{}", message),
        "info" => tracing::info!("{}", message),
        "warn" => tracing::warn!("{}", message),
        "error" => tracing::error!("{}", message),
        _ => tracing::info!("{}", message),
    }
    Ok(Value::new_none())
}