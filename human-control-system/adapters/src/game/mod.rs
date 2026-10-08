//! Game Memory Adapter Module
//!
//! Provides high-level interfaces for game memory manipulation.

// Real process-memory access, scanning and injection modules (memory.rs, scanner.rs, injector.rs) are intentionally not compiled.

use crate::adapters_proto::*;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Game adapter trait
#[async_trait::async_trait]
pub trait GameAdapter: Send + Sync {
    // Process attachment
    async fn attach_process(&self, req: AttachProcessRequest) -> Result<AttachResponse>;
    async fn detach_process(&self, req: DetachProcessRequest) -> Result<()>;
    async fn get_process_info(&self, req: GetProcessInfoRequest) -> Result<ProcessInfoResponse>;
    async fn list_processes(&self, req: ListProcessesRequest) -> Result<ProcessList>;
    async fn find_process_by_name(&self, req: FindProcessByNameRequest) -> Result<ProcessInfoResponse>;
    async fn find_process_by_window(&self, req: FindProcessByWindowRequest) -> Result<ProcessInfoResponse>;

    // Memory reading
    async fn read_memory(&self, req: ReadMemoryRequest) -> Result<ReadMemoryResponse>;
    async fn read_pointer_chain(&self, req: ReadPointerChainRequest) -> Result<ReadMemoryResponse>;
    async fn read_string(&self, req: ReadStringRequest) -> Result<ReadStringResponse>;
    async fn read_struct(&self, req: ReadStructRequest) -> Result<ReadStructResponse>;

    // Memory writing
    async fn write_memory(&self, req: WriteMemoryRequest) -> Result<WriteMemoryResponse>;
    async fn write_pointer_chain(&self, req: WritePointerChainRequest) -> Result<WriteMemoryResponse>;
    async fn write_string(&self, req: WriteStringRequest) -> Result<WriteMemoryResponse>;
    async fn write_struct(&self, req: WriteStructRequest) -> Result<WriteMemoryResponse>;

    // Pattern scanning
    async fn scan_pattern(&self, req: ScanPatternRequest) -> Result<ScanPatternResponse>;
    async fn scan_pattern_module(&self, req: ScanPatternModuleRequest) -> Result<ScanPatternResponse>;
    async fn find_pattern(&self, req: FindPatternRequest) -> Result<FindPatternResponse>;

    // Code injection
    async fn allocate_memory(&self, req: AllocateMemoryRequest) -> Result<AllocateMemoryResponse>;
    async fn free_memory(&self, req: FreeMemoryRequest) -> Result<()>;
    async fn inject_code(&self, req: InjectCodeRequest) -> Result<InjectCodeResponse>;
    async fn create_thread(&self, req: CreateThreadRequest) -> Result<CreateThreadResponse>;
    async fn inject_dll(&self, req: InjectDllRequest) -> Result<InjectDllResponse>;
    async fn set_hook(&self, req: SetHookRequest) -> Result<HookResponse>;
    async fn remove_hook(&self, req: RemoveHookRequest) -> Result<HookResponse>;

    // Modules
    async fn get_modules(&self) -> Result<ModuleList>;
    async fn get_module_base(&self, req: GetModuleBaseRequest) -> Result<ModuleBaseResponse>;
    async fn get_module_export(&self, req: GetModuleExportRequest) -> Result<ModuleExportResponse>;
}

/// Game adapter factory
pub struct GameAdapterFactory;

impl GameAdapterFactory {
    /// Create a game adapter based on configuration
    pub async fn create(config: &crate::config::AdaptersConfig) -> Result<Arc<dyn GameAdapter>> {
        let adapter = memory::MemoryAdapter::new(config).await?;
        Ok(Arc::new(adapter))
    }
}

/// Mock adapter for testing
#[derive(Debug)]
pub struct MockGameAdapter {
    attached_pid: Arc<RwLock<Option<u32>>>,
    process_info: Arc<RwLock<Option<ProcessInfo>>>,
    memory: Arc<RwLock<std::collections::HashMap<u64, Vec<u8>>>>,
    modules: Arc<RwLock<Vec<ModuleInfo>>>,
    counter: Arc<std::sync::atomic::AtomicU64>,
}

impl MockGameAdapter {
    pub fn new() -> Self {
        Self {
            attached_pid: Arc::new(RwLock::new(None)),
            process_info: Arc::new(RwLock::new(None)),
            memory: Arc::new(RwLock::new(std::collections::HashMap::new())),
            modules: Arc::new(RwLock::new(vec![
                ModuleInfo {
                    name: "game.exe".to_string(),
                    base_address: 0x400000,
                    size: 0x1000000,
                    path: "C:\\Games\\game.exe".to_string(),
                    is_main: true,
                    arch: Architecture::ArchitectureX64 as i32,
                },
                ModuleInfo {
                    name: "kernel32.dll".to_string(),
                    base_address: 0x7ffe0000,
                    size: 0x200000,
                    path: "C:\\Windows\\System32\\kernel32.dll".to_string(),
                    is_main: false,
                    arch: Architecture::ArchitectureX64 as i32,
                },
            ])),
            counter: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }
}

#[async_trait::async_trait]
impl GameAdapter for MockGameAdapter {
    async fn attach_process(&self, req: AttachProcessRequest) -> Result<AttachResponse> {
        *self.attached_pid.write().await = Some(req.pid);
        *self.process_info.write().await = Some(ProcessInfo {
            pid: req.pid,
            name: "game.exe".to_string(),
            path: "C:\\Games\\game.exe".to_string(),
            base_address: 0x400000,
            arch: Architecture::ArchitectureX64 as i32,
            is_64bit: true,
            parent_pid: 1234,
            session_id: 1,
            memory_usage: 1024 * 1024 * 512,
            command_line: "game.exe".to_string(),
        });
        Ok(AttachResponse {
            success: true,
            pid: req.pid,
            process_name: "game.exe".to_string(),
            error: String::new(),
        })
    }

    async fn detach_process(&self, req: DetachProcessRequest) -> Result<()> {
        *self.attached_pid.write().await = None;
        *self.process_info.write().await = None;
        Ok(())
    }

    async fn get_process_info(&self, _req: GetProcessInfoRequest) -> Result<ProcessInfoResponse> {
        let info = self.process_info.read().await;
        if let Some(info) = info.as_ref() {
            Ok(ProcessInfoResponse { success: true, process: Some(info.clone()), error: String::new() })
        } else {
            Ok(ProcessInfoResponse { success: false, process: None, error: "Not attached".to_string() })
        }
    }

    async fn list_processes(&self, req: ListProcessesRequest) -> Result<ProcessList> {
        let mut processes = vec![
            ProcessInfo {
                pid: 1234,
                name: "game.exe".to_string(),
                path: "C:\\Games\\game.exe".to_string(),
                base_address: 0x400000,
                arch: Architecture::ArchitectureX64 as i32,
                is_64bit: true,
                parent_pid: 1000,
                session_id: 1,
                memory_usage: 1024 * 1024 * 512,
                command_line: "game.exe".to_string(),
            },
            ProcessInfo {
                pid: 5678,
                name: "other.exe".to_string(),
                path: "C:\\Apps\\other.exe".to_string(),
                base_address: 0x400000,
                arch: Architecture::ArchitectureX64 as i32,
                is_64bit: true,
                parent_pid: 1000,
                session_id: 1,
                memory_usage: 1024 * 1024 * 256,
                command_line: "other.exe".to_string(),
            },
        ];

        if !req.name_filter.is_empty() {
            processes.retain(|p| p.name.contains(&req.name_filter));
        }

        Ok(ProcessList { processes })
    }

    async fn find_process_by_name(&self, req: FindProcessByNameRequest) -> Result<ProcessInfoResponse> {
        let list = self.list_processes(ListProcessesRequest { name_filter: req.name }).await?;
        if let Some(proc) = list.processes.into_iter().next() {
            Ok(ProcessInfoResponse { success: true, process: Some(proc), error: String::new() })
        } else {
            Ok(ProcessInfoResponse { success: false, process: None, error: "Process not found".to_string() })
        }
    }

    async fn find_process_by_window(&self, req: FindProcessByWindowRequest) -> Result<ProcessInfoResponse> {
        // Mock: return first process if window title matches
        let list = self.list_processes(ListProcessesRequest { name_filter: String::new() }).await?;
        if let Some(proc) = list.processes.into_iter().next() {
            Ok(ProcessInfoResponse { success: true, process: Some(proc), error: String::new() })
        } else {
            Ok(ProcessInfoResponse { success: false, process: None, error: "Window not found".to_string() })
        }
    }

    async fn read_memory(&self, req: ReadMemoryRequest) -> Result<ReadMemoryResponse> {
        let mem = self.memory.read().await;
        if let Some(data) = mem.get(&req.address) {
            let size = req.size.min(data.len() as u32);
            Ok(ReadMemoryResponse { success: true, data: data[..size as usize].to_vec(), error: String::new() })
        } else {
            // Return zeros for uninitialized memory
            Ok(ReadMemoryResponse { success: true, data: vec![0; req.size as usize], error: String::new() })
        }
    }

    async fn read_pointer_chain(&self, req: ReadPointerChainRequest) -> Result<ReadMemoryResponse> {
        let mut addr = req.base_address;
        for offset in &req.offsets {
            let data = self.read_memory(ReadMemoryRequest {
                pid: req.pid,
                address: addr,
                size: 8,
            }).await?;
            if data.data.len() >= 8 {
                addr = u64::from_le_bytes(data.data[..8].try_into().unwrap()) + offset;
            } else {
                return Ok(ReadMemoryResponse { success: false, data: vec![], error: "Failed to read pointer".to_string() });
            }
        }
        self.read_memory(ReadMemoryRequest { pid: req.pid, address: addr, size: req.size }).await
    }

    async fn read_string(&self, req: ReadStringRequest) -> Result<ReadStringResponse> {
        let data = self.read_memory(ReadMemoryRequest {
            pid: req.pid,
            address: req.address,
            size: req.max_length,
        }).await?;

        let string = match req.encoding() {
            StringEncoding::StringEncodingAscii => {
                String::from_utf8_lossy(&data.data).trim_end_matches('\0').to_string()
            }
            StringEncoding::StringEncodingUtf8 => {
                String::from_utf8_lossy(&data.data).trim_end_matches('\0').to_string()
            }
            StringEncoding::StringEncodingUtf16 | StringEncoding::StringEncodingUtf16le => {
                let utf16: Vec<u16> = data.data.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
                String::from_utf16_lossy(&utf16).trim_end_matches('\0').to_string()
            }
            _ => String::new(),
        };

        Ok(ReadStringResponse { success: true, value: string, error: String::new() })
    }

    async fn read_struct(&self, _req: ReadStructRequest) -> Result<ReadStructResponse> {
        Ok(ReadStructResponse { success: true, json_value: "{}".to_string(), error: String::new() })
    }

    async fn write_memory(&self, req: WriteMemoryRequest) -> Result<WriteMemoryResponse> {
        let mut mem = self.memory.write().await;
        mem.insert(req.address, req.data.clone());
        Ok(WriteMemoryResponse { success: true, bytes_written: req.data.len() as u32, error: String::new() })
    }

    async fn write_pointer_chain(&self, req: WritePointerChainRequest) -> Result<WriteMemoryResponse> {
        let mut addr = req.base_address;
        let offsets = req.offsets.clone();
        for (i, offset) in offsets.iter().enumerate() {
            let data = self.read_memory(ReadMemoryRequest {
                pid: req.pid,
                address: addr,
                size: 8,
            }).await?;
            if data.data.len() >= 8 {
                addr = u64::from_le_bytes(data.data[..8].try_into().unwrap()) + offset;
            } else {
                return Ok(WriteMemoryResponse { success: false, bytes_written: 0, error: "Failed to read pointer".to_string() });
            }
            if i == offsets.len() - 1 {
                // Last offset, write here
                return self.write_memory(WriteMemoryRequest { pid: req.pid, address: addr, data: req.data }).await;
            }
        }
        Ok(WriteMemoryResponse { success: false, bytes_written: 0, error: "No offsets".to_string() })
    }

    async fn write_string(&self, req: WriteStringRequest) -> Result<WriteMemoryResponse> {
        let data = match req.encoding() {
            StringEncoding::StringEncodingAscii | StringEncoding::StringEncodingUtf8 => {
                req.value.as_bytes().to_vec()
            }
            StringEncoding::StringEncodingUtf16 | StringEncoding::StringEncodingUtf16le => {
                req.value.encode_utf16().flat_map(|c| c.to_le_bytes()).collect()
            }
            _ => vec![],
        };
        self.write_memory(WriteMemoryRequest { pid: req.pid, address: req.address, data }).await
    }

    async fn write_struct(&self, _req: WriteStructRequest) -> Result<WriteMemoryResponse> {
        Ok(WriteMemoryResponse { success: true, bytes_written: 0, error: String::new() })
    }

    async fn scan_pattern(&self, req: ScanPatternRequest) -> Result<ScanPatternResponse> {
        // Mock: return some addresses
        let addresses = if req.pattern.contains("48 8B 05") {
            vec![0x401000, 0x402000, 0x403000]
        } else {
            vec![0x405000]
        };
        Ok(ScanPatternResponse { success: true, addresses, error: String::new() })
    }

    async fn scan_pattern_module(&self, req: ScanPatternModuleRequest) -> Result<ScanPatternResponse> {
        self.scan_pattern(ScanPatternRequest {
            pid: req.pid,
            pattern: req.pattern,
            start_address: 0,
            end_address: 0,
            options: req.options,
        }).await
    }

    async fn find_pattern(&self, req: FindPatternRequest) -> Result<FindPatternResponse> {
        // Simple pattern matching
        let mem = self.memory.read().await;
        let mut addresses = Vec::new();
        for (addr, data) in mem.iter() {
            if data.windows(req.pattern.len()).any(|w| {
                req.mask.chars().zip(w.iter().zip(req.pattern.iter())).all(|(m, (a, b))| m == '?' || a == b)
            }) {
                addresses.push(*addr);
            }
        }
        Ok(FindPatternResponse { success: true, addresses, error: String::new() })
    }

    async fn allocate_memory(&self, req: AllocateMemoryRequest) -> Result<AllocateMemoryResponse> {
        let addr = 0x10000000 + self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) * 0x10000;
        let mut mem = self.memory.write().await;
        mem.insert(addr, vec![0; req.size as usize]);
        Ok(AllocateMemoryResponse { success: true, address: addr, error: String::new() })
    }

    async fn free_memory(&self, req: FreeMemoryRequest) -> Result<()> {
        let mut mem = self.memory.write().await;
        mem.remove(&req.address);
        Ok(())
    }

    async fn inject_code(&self, req: InjectCodeRequest) -> Result<InjectCodeResponse> {
        let alloc = self.allocate_memory(AllocateMemoryRequest {
            pid: req.pid,
            size: req.shellcode.len() as u32,
            protection: MemoryProtection::MemoryProtectionReadWriteExecute as i32,
            type_: AllocationType::AllocationTypeCommitReserve as i32,
            preferred_address: req.target_address,
        }).await?;

        self.write_memory(WriteMemoryRequest {
            pid: req.pid,
            address: alloc.address,
            data: req.shellcode,
        }).await?;

        let thread_id = if req.options.create_thread {
            let thread = self.create_thread(CreateThreadRequest {
                pid: req.pid,
                start_address: alloc.address,
                parameter: req.options.parameter,
                suspended: req.options.suspend_thread,
            }).await?;
            thread.thread_id
        } else {
            0
        };

        Ok(InjectCodeResponse {
            success: true,
            allocated_address: alloc.address,
            thread_id,
            error: String::new(),
        })
    }

    async fn create_thread(&self, _req: CreateThreadRequest) -> Result<CreateThreadResponse> {
        Ok(CreateThreadResponse {
            success: true,
            thread_id: self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) as u32,
            thread_handle: 0,
            error: String::new(),
        })
    }

    async fn inject_dll(&self, req: InjectDllRequest) -> Result<InjectDllResponse> {
        Ok(InjectDllResponse {
            success: true,
            base_address: 0x70000000,
            error: String::new(),
        })
    }

    async fn set_hook(&self, req: SetHookRequest) -> Result<HookResponse> {
        let hook_id = self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(HookResponse {
            success: true,
            hook_id,
            original_bytes: vec![0x90; req.options.original_bytes_size as usize],
            error: String::new(),
        })
    }

    async fn remove_hook(&self, req: RemoveHookRequest) -> Result<HookResponse> {
        Ok(HookResponse { success: true, hook_id: req.hook_id, original_bytes: vec![], error: String::new() })
    }

    async fn get_modules(&self) -> Result<ModuleList> {
        Ok(ModuleList { modules: self.modules.read().await.clone() })
    }

    async fn get_module_base(&self, req: GetModuleBaseRequest) -> Result<ModuleBaseResponse> {
        let modules = self.modules.read().await;
        if let Some(module) = modules.iter().find(|m| m.name == req.module_name) {
            Ok(ModuleBaseResponse { success: true, base_address: module.base_address, error: String::new() })
        } else {
            Ok(ModuleBaseResponse { success: false, base_address: 0, error: "Module not found".to_string() })
        }
    }

    async fn get_module_export(&self, req: GetModuleExportRequest) -> Result<ModuleExportResponse> {
        // Mock: return a fake export address
        Ok(ModuleExportResponse {
            success: true,
            address: 0x7ffe001000,
            error: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_game_adapter() {
        let adapter = MockGameAdapter::new();

        let attach = adapter.attach_process(AttachProcessRequest { pid: 1234, access: AccessRights::AccessRightsReadWrite as i32 }).await.unwrap();
        assert!(attach.success);

        let info = adapter.get_process_info(GetProcessInfoRequest { pid: 1234 }).await.unwrap();
        assert!(info.success);
        assert_eq!(info.process.as_ref().unwrap().pid, 1234);

        let read = adapter.read_memory(ReadMemoryRequest { pid: 1234, address: 0x400000, size: 16 }).await.unwrap();
        assert!(read.success);
        assert_eq!(read.data.len(), 16);
    }

    #[tokio::test]
    async fn test_mock_game_adapter_write_read() {
        let adapter = MockGameAdapter::new();
        adapter.attach_process(AttachProcessRequest { pid: 1234, access: AccessRights::AccessRightsReadWrite as i32 }).await.unwrap();

        let write = adapter.write_memory(WriteMemoryRequest { pid: 1234, address: 0x500000, data: vec![1, 2, 3, 4] }).await.unwrap();
        assert!(write.success);
        assert_eq!(write.bytes_written, 4);

        let read = adapter.read_memory(ReadMemoryRequest { pid: 1234, address: 0x500000, size: 4 }).await.unwrap();
        assert!(read.success);
        assert_eq!(read.data, vec![1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn test_mock_game_adapter_pattern_scan() {
        let adapter = MockGameAdapter::new();
        adapter.attach_process(AttachProcessRequest { pid: 1234, access: AccessRights::AccessRightsReadWrite as i32 }).await.unwrap();

        let scan = adapter.scan_pattern(ScanPatternRequest {
            pid: 1234,
            pattern: "48 8B 05 ?? ?? ?? ??".to_string(),
            start_address: 0x400000,
            end_address: 0x500000,
            options: None,
        }).await.unwrap();
        assert!(scan.success);
        assert!(!scan.addresses.is_empty());
    }
}