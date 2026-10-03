//! Host Functions for WASM Plugins

use anyhow::Result;
use wasmtime::{Caller, Func, Linker, Memory, Store, Val, ValType};
use std::sync::Arc;
use parking_lot::RwLock;
use crate::bt::Blackboard;
use crate::wasm::runtime::WasiState;

/// Host functions exposed to WASM plugins
pub struct HostFunctions {
    blackboard: Arc<RwLock<Blackboard>>,
}

impl HostFunctions {
    pub fn new(blackboard: Arc<RwLock<Blackboard>>) -> Self {
        Self { blackboard }
    }
    
    /// Add all host functions to a linker
    pub fn add_to_linker(&self, linker: &mut Linker<WasiState>) -> Result<()> {
        let blackboard = self.blackboard.clone();
        
        // blackboard_get(key: string) -> string (JSON)
        linker.func_wrap("host", "blackboard_get", move |mut caller: Caller<'_, WasiState>, key_ptr: i32, key_len: i32, out_ptr: i32, out_len_ptr: i32| -> i32 {
            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(mem)) => mem,
                _ => return -1,
            };
            
            // Read key from WASM memory
            let key = match read_string_from_memory(&memory, &caller, key_ptr, key_len) {
                Ok(k) => k,
                Err(_) => return -1,
            };
            
            // Get value from blackboard
            let bb = blackboard.read();
            let value = bb.get(&key);
            let json = match value {
                Some(v) => serde_json::to_string(&v).unwrap_or_default(),
                None => "null".to_string(),
            };
            
            // Write result to output buffer
            match write_string_to_memory(&memory, &mut caller, out_ptr, out_len_ptr, &json) {
                Ok(_) => 0,
                Err(_) => -1,
            }
        })?;
        
        let blackboard = self.blackboard.clone();
        // blackboard_set(key: string, value: string (JSON)) -> bool
        linker.func_wrap("host", "blackboard_set", move |mut caller: Caller<'_, WasiState>, key_ptr: i32, key_len: i32, val_ptr: i32, val_len: i32| -> i32 {
            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(mem)) => mem,
                _ => return -1,
            };
            
            let key = match read_string_from_memory(&memory, &caller, key_ptr, key_len) {
                Ok(k) => k,
                Err(_) => return -1,
            };
            
            let val_json = match read_string_from_memory(&memory, &caller, val_ptr, val_len) {
                Ok(v) => v,
                Err(_) => return -1,
            };
            
            let value: serde_json::Value = match serde_json::from_str(&val_json) {
                Ok(v) => v,
                Err(_) => return -1,
            };
            
            blackboard.write().set(key, value);
            1
        })?;
        
        let blackboard = self.blackboard.clone();
        // blackboard_has(key: string) -> bool
        linker.func_wrap("host", "blackboard_has", move |mut caller: Caller<'_, WasiState>, key_ptr: i32, key_len: i32| -> i32 {
            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(mem)) => mem,
                _ => return -1,
            };
            
            let key = match read_string_from_memory(&memory, &caller, key_ptr, key_len) {
                Ok(k) => k,
                Err(_) => return -1,
            };
            
            if blackboard.read().has(&key) { 1 } else { 0 }
        })?;
        
        // log_info(message: string)
        linker.func_wrap("host", "log_info", move |mut caller: Caller<'_, WasiState>, msg_ptr: i32, msg_len: i32| {
            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(mem)) => mem,
                _ => return,
            };
            
            if let Ok(msg) = read_string_from_memory(&memory, &caller, msg_ptr, msg_len) {
                tracing::info!(target: "wasm_plugin", message = %msg);
            }
        })?;
        
        // log_warn(message: string)
        linker.func_wrap("host", "log_warn", move |mut caller: Caller<'_, WasiState>, msg_ptr: i32, msg_len: i32| {
            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(mem)) => mem,
                _ => return,
            };
            
            if let Ok(msg) = read_string_from_memory(&memory, &caller, msg_ptr, msg_len) {
                tracing::warn!(target: "wasm_plugin", message = %msg);
            }
        })?;
        
        // log_error(message: string)
        linker.func_wrap("host", "log_error", move |mut caller: Caller<'_, WasiState>, msg_ptr: i32, msg_len: i32| {
            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(mem)) => mem,
                _ => return,
            };
            
            if let Ok(msg) = read_string_from_memory(&memory, &caller, msg_ptr, msg_len) {
                tracing::error!(target: "wasm_plugin", message = %msg);
            }
        })?;
        
        // get_time_ms() -> i64
        linker.func_wrap("host", "get_time_ms", || -> i64 {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64
        })?;
        
        // random_float() -> f64
        linker.func_wrap("host", "random_float", || -> f64 {
            use rand::Rng;
            rand::thread_rng().gen()
        })?;
        
        // random_int(min: i32, max: i32) -> i32
        linker.func_wrap("host", "random_int", |min: i32, max: i32| -> i32 {
            use rand::Rng;
            rand::thread_rng().gen_range(min..max)
        })?;
        
        // sleep_ms(ms: i32)
        linker.func_wrap("host", "sleep_ms", |ms: i32| {
            std::thread::sleep(std::time::Duration::from_millis(ms as u64));
        })?;
        
        Ok(())
    }
}

/// Read a string from WASM linear memory
fn read_string_from_memory(memory: &Memory, caller: &Caller<'_, WasiState>, ptr: i32, len: i32) -> Result<String> {
    let data = memory.data(caller);
    let start = usize::try_from(ptr)?;
    let end = start.checked_add(usize::try_from(len)?).ok_or_else(|| anyhow::anyhow!("Buffer overflow"))?;
    
    if end > data.len() {
        return Err(anyhow::anyhow!("Memory access out of bounds"));
    }
    
    let bytes = &data[start..end];
    Ok(String::from_utf8_lossy(bytes).to_string())
}

/// Write a string to WASM linear memory
fn write_string_to_memory(memory: &Memory, caller: &mut Caller<'_, WasiState>, ptr: i32, len_ptr: i32, s: &str) -> Result<()> {
    let data = memory.data_mut(caller);
    let start = usize::try_from(ptr)?;
    let length_index = usize::try_from(len_ptr)?;
    let length_end = length_index.checked_add(4).ok_or_else(|| anyhow::anyhow!("Invalid length pointer"))?;
    let capacity = i32::from_le_bytes(data.get(length_index..length_end)
        .ok_or_else(|| anyhow::anyhow!("Length pointer out of bounds"))?.try_into()?);
    let capacity = usize::try_from(capacity)?;
    let end = start.checked_add(s.len()).ok_or_else(|| anyhow::anyhow!("Buffer overflow"))?;
    if s.len() > capacity || end > data.len() { anyhow::bail!("Output buffer too small"); }
    data[start..end].copy_from_slice(s.as_bytes());
    data[length_index..length_end].copy_from_slice(&(s.len() as i32).to_le_bytes());
    Ok(())
}

/// Create a default host functions instance with empty blackboard
pub fn create_default_host_functions() -> HostFunctions {
    HostFunctions::new(Arc::new(RwLock::new(Blackboard::new())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmtime::{Engine, Module, Store};
    use crate::bt::Blackboard;
    
    #[test]
    fn test_host_functions_creation() {
        let hf = create_default_host_functions();
        // Just verify it creates without error
    }
    
    #[tokio::test]
    async fn test_memory_operations() {
        let engine = Engine::default();
        let mut store = Store::new(&engine, WasiState::new());
        
        // Create a simple module with memory
        let wat = r#"
            (module
                (memory 1)
                (export "memory" (memory 0))
            )
        "#;
        
        let module = Module::new(&engine, wat).unwrap();
        let mut linker = Linker::new(&engine);
        
        let hf = create_default_host_functions();
        hf.add_to_linker(&mut linker).unwrap();
        
        let instance = linker.instantiate(&mut store, &module).unwrap();
        
        // Verify memory export exists
        let memory = instance.get_export(&mut store, "memory");
        assert!(memory.is_some());
    }
}