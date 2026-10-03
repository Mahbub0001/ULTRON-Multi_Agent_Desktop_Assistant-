//! Memory Adapter for Game Processes
//!
//! Provides low-level memory reading/writing capabilities.

use crate::game::{GameAdapter, MockGameAdapter};
use crate::adapters_proto::*;
use crate::config::AdaptersConfig;
use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info, warn};

#[cfg(target_os = "windows")]
use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
#[cfg(target_os = "windows")]
use windows::Win32::System::Memory::{VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, PAGE_EXECUTE_READWRITE};
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::{OpenProcess, PROCESS_VM_READ, PROCESS_VM_WRITE, PROCESS_VM_OPERATION, PROCESS_CREATE_THREAD, PROCESS_QUERY_INFORMATION};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HANDLE, CloseHandle};

/// Platform-specific memory operations
#[derive(Debug)]
pub struct MemoryAdapter {
    config: AdaptersConfig,
    attached_pid: Arc<tokio::sync::RwLock<Option<u32>>>,
    #[cfg(target_os = "windows")]
    process_handle: Arc<tokio::sync::RwLock<Option<HANDLE>>>,
    // In real implementation, this would hold the actual process handle
    // For now, we delegate to mock
    mock: Arc<MockGameAdapter>,
}

impl MemoryAdapter {
    /// Create a new memory adapter
    pub async fn new(config: &AdaptersConfig) -> Result<Self> {
        info!("Initializing Memory adapter");

        let mock = Arc::new(MockGameAdapter::new());

        Ok(Self {
            config: config.clone(),
            attached_pid: Arc::new(tokio::sync::RwLock::new(None)),
            #[cfg(target_os = "windows")]
            process_handle: Arc::new(tokio::sync::RwLock::new(None)),
            mock,
        })
    }

    /// Attach to a process
    pub async fn attach(&self, pid: u32, access: AccessRights) -> Result<AttachResponse> {
        // In a real implementation:
        // Windows: OpenProcess with appropriate access rights
        // Linux: ptrace attach or process_vm_readv/writev

        *self.attached_pid.write().await = Some(pid);

        #[cfg(target_os = "windows")]
        {
            let access_rights = match access {
                AccessRights::AccessRightsRead => PROCESS_VM_READ | PROCESS_QUERY_INFORMATION,
                AccessRights::AccessRightsWrite => PROCESS_VM_WRITE | PROCESS_VM_OPERATION,
                AccessRights::AccessRightsReadWrite => PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION | PROCESS_QUERY_INFORMATION,
                AccessRights::AccessRightsAll => PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION | PROCESS_CREATE_THREAD | PROCESS_QUERY_INFORMATION,
                _ => PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION,
            };

            unsafe {
                let handle = OpenProcess(access_rights, false, pid)?;
                *self.process_handle.write().await = Some(handle);
            }
        }

        info!(pid, "Attached to process");
        self.mock.attach_process(AttachProcessRequest { pid, access: access as i32 }).await
    }

    /// Detach from process
    pub async fn detach(&self) -> Result<()> {
        if let Some(pid) = *self.attached_pid.read().await {
            #[cfg(target_os = "windows")]
            {
                if let Some(handle) = *self.process_handle.read().await {
                    unsafe { CloseHandle(handle)?; }
                    *self.process_handle.write().await = None;
                }
            }
            *self.attached_pid.write().await = None;
            info!(pid, "Detached from process");
        }
        self.mock.detach_process(DetachProcessRequest { pid: 0 }).await
    }

    /// Read memory from process
    pub async fn read(&self, address: u64, size: usize) -> Result<Vec<u8>> {
        let pid = *self.attached_pid.read().await
            .ok_or_else(|| anyhow::anyhow!("Not attached to any process"))?;

        #[cfg(target_os = "windows")]
        {
            if let Some(handle) = *self.process_handle.read().await {
                let mut buffer = vec![0u8; size];
                let mut bytes_read = 0;
                unsafe {
                    ReadProcessMemory(
                        handle,
                        address as _,
                        buffer.as_mut_ptr() as _,
                        size,
                        Some(&mut bytes_read),
                    )?;
                }
                buffer.truncate(bytes_read);
                return Ok(buffer);
            }
        }

        // Fallback to mock
        let resp = self.mock.read_memory(ReadMemoryRequest { pid, address, size: size as u32 }).await?;
        Ok(resp.data)
    }

    /// Write memory to process
    pub async fn write(&self, address: u64, data: &[u8]) -> Result<usize> {
        let pid = *self.attached_pid.read().await
            .ok_or_else(|| anyhow::anyhow!("Not attached to any process"))?;

        #[cfg(target_os = "windows")]
        {
            if let Some(handle) = *self.process_handle.read().await {
                use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
                let mut bytes_written = 0;
                unsafe {
                    WriteProcessMemory(
                        handle,
                        address as _,
                        data.as_ptr() as _,
                        data.len(),
                        Some(&mut bytes_written),
                    )?;
                }
                return Ok(bytes_written);
            }
        }

        // Fallback to mock
        let resp = self.mock.write_memory(WriteMemoryRequest { pid, address, data: data.to_vec() }).await?;
        Ok(resp.bytes_written as usize)
    }

    /// Read a pointer chain
    pub async fn read_pointer_chain(&self, base: u64, offsets: &[u64], size: usize) -> Result<Vec<u8>> {
        let mut addr = base;
        for (i, offset) in offsets.iter().enumerate() {
            let ptr_data = self.read(addr, 8).await?;
            if ptr_data.len() < 8 {
                return Err(anyhow::anyhow!("Failed to read pointer at offset {}", i));
            }
            addr = u64::from_le_bytes(ptr_data[..8].try_into().unwrap()) + offset;
        }
        self.read(addr, size).await
    }

    /// Write a pointer chain
    pub async fn write_pointer_chain(&self, base: u64, offsets: &[u64], data: &[u8]) -> Result<usize> {
        let mut addr = base;
        for (i, offset) in offsets.iter().enumerate() {
            let ptr_data = self.read(addr, 8).await?;
            if ptr_data.len() < 8 {
                return Err(anyhow::anyhow!("Failed to read pointer at offset {}", i));
            }
            addr = u64::from_le_bytes(ptr_data[..8].try_into().unwrap()) + offset;

            // If this is the last offset, write the data
            if i == offsets.len() - 1 {
                return self.write(addr, data).await;
            }
        }
        Err(anyhow::anyhow!("No offsets provided"))
    }

    /// Allocate memory in target process
    pub async fn allocate(&self, size: usize, protection: MemoryProtection) -> Result<u64> {
        let pid = *self.attached_pid.read().await
            .ok_or_else(|| anyhow::anyhow!("Not attached to any process"))?;

        #[cfg(target_os = "windows")]
        {
            if let Some(handle) = *self.process_handle.read().await {
                let prot = match protection {
                    MemoryProtection::MemoryProtectionRead => windows::Win32::System::Memory::PAGE_READONLY,
                    MemoryProtection::MemoryProtectionWrite => windows::Win32::System::Memory::PAGE_READWRITE,
                    MemoryProtection::MemoryProtectionExecute => windows::Win32::System::Memory::PAGE_EXECUTE,
                    MemoryProtection::MemoryProtectionReadWrite => windows::Win32::System::Memory::PAGE_READWRITE,
                    MemoryProtection::MemoryProtectionReadExecute => windows::Win32::System::Memory::PAGE_EXECUTE_READ,
                    MemoryProtection::MemoryProtectionReadWriteExecute => windows::Win32::System::Memory::PAGE_EXECUTE_READWRITE,
                    _ => windows::Win32::System::Memory::PAGE_READWRITE,
                };

                let addr = unsafe {
                    VirtualAllocEx(handle, None, size, MEM_COMMIT | MEM_RESERVE, prot)?
                };
                return Ok(addr as u64);
            }
        }

        // Fallback to mock
        let resp = self.mock.allocate_memory(AllocateMemoryRequest {
            pid,
            size: size as u32,
            protection: protection as i32,
            type_: AllocationType::AllocationTypeCommitReserve as i32,
            preferred_address: 0,
        }).await?;
        Ok(resp.address)
    }

    /// Free allocated memory
    pub async fn free(&self, address: u64, size: usize) -> Result<()> {
        let pid = *self.attached_pid.read().await
            .ok_or_else(|| anyhow::anyhow!("Not attached to any process"))?;

        #[cfg(target_os = "windows")]
        {
            if let Some(handle) = *self.process_handle.read().await {
                unsafe {
                    VirtualFreeEx(handle, address as _, 0, windows::Win32::System::Memory::MEM_RELEASE)?;
                }
                return Ok(());
            }
        }

        self.mock.free_memory(FreeMemoryRequest { pid, address, size: size as u32 }).await
    }

    /// Check if attached
    pub fn is_attached(&self) -> bool {
        self.attached_pid.try_read().map(|p| p.is_some()).unwrap_or(false)
    }

    /// Get attached PID
    pub async fn get_pid(&self) -> Option<u32> {
        *self.attached_pid.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AdaptersConfig;

    #[tokio::test]
    async fn test_memory_adapter_creation() {
        let config = AdaptersConfig::default();
        let adapter = MemoryAdapter::new(&config).await;
        assert!(adapter.is_ok());
    }

    #[tokio::test]
    async fn test_memory_adapter_mock_operations() {
        let config = AdaptersConfig::default();
        let adapter = MemoryAdapter::new(&config).await.unwrap();

        // Attach via mock
        adapter.mock.attach_process(AttachProcessRequest { pid: 1234, access: AccessRights::AccessRightsReadWrite as i32 }).await.unwrap();

        // Write and read via mock
        adapter.mock.write_memory(WriteMemoryRequest { pid: 1234, address: 0x500000, data: vec![1, 2, 3, 4] }).await.unwrap();
        let read = adapter.mock.read_memory(ReadMemoryRequest { pid: 1234, address: 0x500000, size: 4 }).await.unwrap();
        assert_eq!(read.data, vec![1, 2, 3, 4]);
    }
}