//! Code Injection for Game Processes
//!
//! Provides code injection, hooking, and DLL injection capabilities.

use crate::game::{GameAdapter, MockGameAdapter};
use crate::adapters_proto::*;
use crate::config::AdaptersConfig;
use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Code injector for game processes
#[derive(Debug)]
pub struct CodeInjector {
    config: AdaptersConfig,
    adapter: Arc<dyn GameAdapter>,
    hooks: Arc<tokio::sync::RwLock<std::collections::HashMap<u64, HookInfo>>>,
}

#[derive(Debug, Clone)]
struct HookInfo {
    id: u64,
    target_address: u64,
    detour_address: u64,
    original_bytes: Vec<u8>,
    hook_type: HookType,
    enabled: bool,
}

impl CodeInjector {
    /// Create a new code injector
    pub fn new(adapter: Arc<dyn GameAdapter>) -> Self {
        Self {
            config: AdaptersConfig::default(),
            adapter,
            hooks: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Inject shellcode and optionally execute it
    pub async fn inject_shellcode(&self, pid: u32, shellcode: &[u8], options: InjectOptions) -> Result<InjectCodeResponse> {
        let req = InjectCodeRequest {
            pid,
            shellcode: shellcode.to_vec(),
            target_address: 0,
            options: Some(options),
        };
        self.adapter.inject_code(req).await
    }

    /// Allocate and write shellcode, return address
    pub async fn allocate_and_write(&self, pid: u32, shellcode: &[u8], protection: MemoryProtection) -> Result<u64> {
        let addr = self.adapter.allocate_memory(AllocateMemoryRequest {
            pid,
            size: shellcode.len() as u32,
            protection: protection as i32,
            type_: AllocationType::AllocationTypeCommitReserve as i32,
            preferred_address: 0,
        }).await?.address;

        self.adapter.write_memory(WriteMemoryRequest {
            pid,
            address: addr,
            data: shellcode.to_vec(),
        }).await?;

        Ok(addr)
    }

    /// Create a remote thread at address
    pub async fn create_remote_thread(&self, pid: u32, start_address: u64, parameter: u64, suspended: bool) -> Result<u32> {
        let resp = self.adapter.create_thread(CreateThreadRequest {
            pid,
            start_address,
            parameter,
            suspended,
        }).await?;
        Ok(resp.thread_id)
    }

    /// Inject a DLL into target process
    pub async fn inject_dll(&self, pid: u32, dll_path: &str, method: InjectionMethod) -> Result<InjectDllResponse> {
        let req = InjectDllRequest {
            pid,
            dll_path: dll_path.to_string(),
            method: method as i32,
        };
        self.adapter.inject_dll(req).await
    }

    /// Set an inline hook (JMP hook)
    pub async fn set_inline_hook(&self, pid: u32, target: u64, detour: u64) -> Result<u64> {
        // Read original bytes (at least 14 bytes for x64 JMP)
        let original = self.adapter.read_memory(ReadMemoryRequest {
            pid,
            address: target,
            size: 14,
        }).await?;

        // Build JMP instruction (x64: FF 25 00 00 00 00 + 8-byte address)
        let mut jmp = vec![0xFF, 0x25, 0x00, 0x00, 0x00, 0x00]; // JMP [RIP+0]
        jmp.extend_from_slice(&detour.to_le_bytes());

        // Write hook
        self.adapter.write_memory(WriteMemoryRequest {
            pid,
            address: target,
            data: jmp,
        }).await?;

        let hook_id = self.adapter.set_hook(SetHookRequest {
            pid,
            target_address: target,
            detour_address: detour,
            type_: HookType::HookTypeJmp as i32,
            options: Some(HookOptions { auto_enable: true, original_bytes_size: 14 }),
        }).await?.hook_id;

        // Store hook info
        let mut hooks = self.hooks.write().await;
        hooks.insert(hook_id, HookInfo {
            id: hook_id,
            target_address: target,
            detour_address: detour,
            original_bytes: original.data,
            hook_type: HookType::HookTypeJmp,
            enabled: true,
        });

        Ok(hook_id)
    }

    /// Set a VEH (Vectored Exception Handler) hook
    pub async fn set_veh_hook(&self, pid: u32, target: u64, detour: u64) -> Result<u64> {
        // VEH hooks require kernel-mode or special setup
        // For user-mode, we use a different approach
        let hook_id = self.adapter.set_hook(SetHookRequest {
            pid,
            target_address: target,
            detour_address: detour,
            type_: HookType::HookTypeVeh as i32,
            options: Some(HookOptions { auto_enable: true, original_bytes_size: 1 }),
        }).await?.hook_id;

        let mut hooks = self.hooks.write().await;
        hooks.insert(hook_id, HookInfo {
            id: hook_id,
            target_address: target,
            detour_address: detour,
            original_bytes: vec![],
            hook_type: HookType::HookTypeVeh,
            enabled: true,
        });

        Ok(hook_id)
    }

    /// Set an IAT (Import Address Table) hook
    pub async fn set_iat_hook(&self, pid: u32, module_name: &str, import_name: &str, detour: u64) -> Result<u64> {
        // Get module base
        let module_base = self.adapter.get_module_base(GetModuleBaseRequest {
            pid,
            module_name: module_name.to_string(),
        }).await?.base_address;

        if module_base == 0 {
            return Err(anyhow::anyhow!("Module not found: {}", module_name));
        }

        // In a real implementation, parse PE headers to find IAT entry
        // For now, return mock hook ID
        let hook_id = self.adapter.set_hook(SetHookRequest {
            pid,
            target_address: module_base,
            detour_address: detour,
            type_: HookType::HookTypeIat as i32,
            options: Some(HookOptions { auto_enable: true, original_bytes_size: 8 }),
        }).await?.hook_id;

        let mut hooks = self.hooks.write().await;
        hooks.insert(hook_id, HookInfo {
            id: hook_id,
            target_address: module_base,
            detour_address: detour,
            original_bytes: vec![],
            hook_type: HookType::HookTypeIat,
            enabled: true,
        });

        Ok(hook_id)
    }

    /// Remove a hook and restore original bytes
    pub async fn remove_hook(&self, pid: u32, hook_id: u64) -> Result<()> {
        let mut hooks = self.hooks.write().await;
        if let Some(hook) = hooks.remove(&hook_id) {
            if hook.enabled && !hook.original_bytes.is_empty() {
                // Restore original bytes
                self.adapter.write_memory(WriteMemoryRequest {
                    pid,
                    address: hook.target_address,
                    data: hook.original_bytes,
                }).await?;
            }
            self.adapter.remove_hook(RemoveHookRequest { pid, hook_id }).await?;
        }
        Ok(())
    }

    /// Enable/disable a hook
    pub async fn toggle_hook(&self, pid: u32, hook_id: u64, enable: bool) -> Result<()> {
        let mut hooks = self.hooks.write().await;
        if let Some(hook) = hooks.get_mut(&hook_id) {
            if hook.enabled == enable {
                return Ok(());
            }

            if enable {
                // Re-apply hook
                let jmp = build_jmp_instruction(hook.detour_address);
                self.adapter.write_memory(WriteMemoryRequest {
                    pid,
                    address: hook.target_address,
                    data: jmp,
                }).await?;
            } else {
                // Restore original
                if !hook.original_bytes.is_empty() {
                    self.adapter.write_memory(WriteMemoryRequest {
                        pid,
                        address: hook.target_address,
                        data: hook.original_bytes.clone(),
                    }).await?;
                }
            }
            hook.enabled = enable;
        }
        Ok(())
    }

    /// Get hook info
    pub async fn get_hook(&self, hook_id: u64) -> Option<HookInfo> {
        self.hooks.read().await.get(&hook_id).cloned()
    }

    /// List all hooks
    pub async fn list_hooks(&self) -> Vec<HookInfo> {
        self.hooks.read().await.values().cloned().collect()
    }
}

/// Build x64 JMP [RIP+0] instruction with 8-byte address
fn build_jmp_instruction(target: u64) -> Vec<u8> {
    let mut jmp = vec![0xFF, 0x25, 0x00, 0x00, 0x00, 0x00]; // JMP [RIP+0]
    jmp.extend_from_slice(&target.to_le_bytes());
    jmp
}

/// Shellcode generator for common operations
pub mod shellcode {
    /// Generate shellcode to call a function with parameters
    pub fn call_function(addr: u64, args: &[u64], convention: CallConvention) -> Vec<u8> {
        match convention {
            CallConvention::SystemVAmd64 => call_system_v_amd64(addr, args),
            CallConvention::MicrosoftX64 => call_microsoft_x64(addr, args),
            CallConvention::Cdecl => call_cdecl(addr, args),
        }
    }

    /// System V AMD64 ABI (Linux/macOS)
    fn call_system_v_amd64(addr: u64, args: &[u64]) -> Vec<u8> {
        let mut code = Vec::new();

        // Save registers we'll clobber
        code.extend([0x50, 0x51, 0x52, 0x53, 0x56, 0x57, 0x41, 0x50, 0x41, 0x51, 0x41, 0x52, 0x41, 0x53]); // push rax,rcx,rdx,rbx,rsi,rdi,r8-r15

        // Set up arguments (RDI, RSI, RDX, RCX, R8, R9, then stack)
        let regs = [0xF7, 0xF6, 0xF2, 0xF1, 0xF0, 0xE8]; // RDI, RSI, RDX, RCX, R8, R9 (MOV reg, imm64)
        for (i, arg) in args.iter().enumerate() {
            if i < 6 {
                code.push(0x48); // REX.W
                code.push(regs[i]); // MOV reg, imm64
                code.extend_from_slice(&arg.to_le_bytes());
            } else {
                // Push to stack (right to left)
                code.push(0x68); // PUSH imm32 (sign-extended)
                code.extend_from_slice(&(arg as u32).to_le_bytes());
            }
        }

        // Call function
        code.push(0x48); // REX.W
        code.push(0xB8); // MOV RAX, imm64
        code.extend_from_slice(&addr.to_le_bytes());
        code.push(0xFF); // CALL RAX
        code.push(0xD0);

        // Restore registers
        code.extend([0x41, 0x5B, 0x41, 0x5A, 0x41, 0x59, 0x41, 0x58, 0x5F, 0x5E, 0x5B, 0x5A, 0x59, 0x58]); // pop r15-r8,rdi,rsi,rdx,rbx,rcx,rax

        // RET
        code.push(0xC3);

        code
    }

    /// Microsoft x64 ABI (Windows)
    fn call_microsoft_x64(addr: u64, args: &[u64]) -> Vec<u8> {
        let mut code = Vec::new();

        // Allocate shadow space (32 bytes) + align stack
        code.extend([0x48, 0x83, 0xEC, 0x28]); // SUB RSP, 0x28

        // Save non-volatile registers
        code.extend([0x53, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57]); // push rbx,rsi,rdi,r12-r15

        // Set up arguments (RCX, RDX, R8, R9, then stack)
        let regs = [0xC1, 0xC2, 0xC0, 0xC8]; // RCX, RDX, R8, R9
        for (i, arg) in args.iter().enumerate() {
            if i < 4 {
                code.push(0x48); // REX.W
                code.push(0xB8 + regs[i]); // MOV reg, imm64
                code.extend_from_slice(&arg.to_le_bytes());
            } else {
                // Stack arguments
                let offset = 0x28 + (i - 4) * 8;
                code.push(0x48); // REX.W
                code.push(0xC7); // MOV [RSP+offset], imm32
                code.push(0x44);
                code.push(0x24);
                code.push(offset as u8);
                code.extend_from_slice(&(arg as u32).to_le_bytes());
            }
        }

        // Call function
        code.push(0x48); // REX.W
        code.push(0xB8); // MOV RAX, imm64
        code.extend_from_slice(&addr.to_le_bytes());
        code.push(0xFF); // CALL RAX
        code.push(0xD0);

        // Restore non-volatile registers
        code.extend([0x41, 0x5F, 0x41, 0x5E, 0x41, 0x5D, 0x41, 0x5C, 0x5F, 0x5E, 0x5B]); // pop r15-r12,rdi,rsi,rbx

        // Restore stack
        code.extend([0x48, 0x83, 0xC4, 0x28]); // ADD RSP, 0x28

        // RET
        code.push(0xC3);

        code
    }

    /// CDECL (32-bit) - not applicable for 64-bit
    fn call_cdecl(_addr: u64, _args: &[u64]) -> Vec<u8> {
        vec![] // Not implemented for 64-bit
    }

    /// Generate shellcode for a simple memory write
    pub fn write_memory(addr: u64, value: u64, size: u8) -> Vec<u8> {
        let mut code = Vec::new();
        code.push(0x48); // REX.W
        code.push(0xB8); // MOV RAX, imm64 (address)
        code.extend_from_slice(&addr.to_le_bytes());

        match size {
            1 => {
                code.push(0xC6); // MOV BYTE PTR [RAX], imm8
                code.push(0x00);
                code.push(value as u8);
            }
            2 => {
                code.push(0x66);
                code.push(0xC7); // MOV WORD PTR [RAX], imm16
                code.push(0x00);
                code.extend_from_slice(&(value as u16).to_le_bytes());
            }
            4 => {
                code.push(0xC7); // MOV DWORD PTR [RAX], imm32
                code.push(0x00);
                code.extend_from_slice(&(value as u32).to_le_bytes());
            }
            8 => {
                code.push(0x48);
                code.push(0xC7); // MOV QWORD PTR [RAX], imm64
                code.push(0x00);
                code.extend_from_slice(&value.to_le_bytes());
            }
            _ => {}
        }

        code.push(0xC3); // RET
        code
    }

    /// Generate shellcode for a memory read (returns value in RAX)
    pub fn read_memory(addr: u64, size: u8) -> Vec<u8> {
        let mut code = Vec::new();
        code.push(0x48); // REX.W
        code.push(0xB8); // MOV RAX, imm64 (address)
        code.extend_from_slice(&addr.to_le_bytes());

        match size {
            1 => {
                code.push(0x0F);
                code.push(0xB6); // MOVZX RAX, BYTE PTR [RAX]
                code.push(0x00);
            }
            2 => {
                code.push(0x0F);
                code.push(0xB7); // MOVZX RAX, WORD PTR [RAX]
                code.push(0x00);
            }
            4 => {
                code.push(0x8B); // MOV EAX, DWORD PTR [RAX]
                code.push(0x00);
            }
            8 => {
                code.push(0x48);
                code.push(0x8B); // MOV RAX, QWORD PTR [RAX]
                code.push(0x00);
            }
            _ => {}
        }

        code.push(0xC3); // RET
        code
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CallConvention {
    SystemVAmd64,
    MicrosoftX64,
    Cdecl,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_jmp_instruction() {
        let jmp = build_jmp_instruction(0x123456789ABCDEF0);
        assert_eq!(jmp.len(), 14);
        assert_eq!(jmp[0], 0xFF);
        assert_eq!(jmp[1], 0x25);
        assert_eq!(jmp[2..8], [0, 0, 0, 0, 0, 0]);
        assert_eq!(jmp[8..], 0x123456789ABCDEF0u64.to_le_bytes());
    }

    #[test]
    fn test_shellcode_write_memory() {
        let code = shellcode::write_memory(0x1000, 0x12345678, 4);
        assert!(!code.is_empty());
        assert_eq!(code.last(), Some(&0xC3)); // RET
    }

    #[test]
    fn test_shellcode_read_memory() {
        let code = shellcode::read_memory(0x1000, 8);
        assert!(!code.is_empty());
        assert_eq!(code.last(), Some(&0xC3)); // RET
    }

    #[test]
    fn test_call_convention_system_v() {
        let code = shellcode::call_function(0x1000, &[1, 2, 3], CallConvention::SystemVAmd64);
        assert!(!code.is_empty());
        assert_eq!(code.last(), Some(&0xC3));
    }

    #[test]
    fn test_call_convention_microsoft() {
        let code = shellcode::call_function(0x1000, &[1, 2, 3], CallConvention::MicrosoftX64);
        assert!(!code.is_empty());
        assert_eq!(code.last(), Some(&0xC3));
    }
}