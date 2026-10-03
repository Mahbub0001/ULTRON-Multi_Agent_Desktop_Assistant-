//! Pattern Scanner for Game Memory
//!
//! Provides pattern scanning capabilities (IDA-style signatures).

use crate::game::{GameAdapter, MockGameAdapter};
use crate::adapters_proto::*;
use crate::config::AdaptersConfig;
use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info};

/// Pattern scanner using IDA-style signatures
#[derive(Debug)]
pub struct PatternScanner {
    config: AdaptersConfig,
    adapter: Arc<dyn GameAdapter>,
}

impl PatternScanner {
    /// Create a new pattern scanner
    pub fn new(adapter: Arc<dyn GameAdapter>) -> Self {
        Self {
            config: AdaptersConfig::default(),
            adapter,
        }
    }

    /// Parse IDA-style pattern string to bytes and mask
    /// Example: "48 8B 05 ?? ?? ?? ?? 48 85 C0" -> bytes=[0x48, 0x8B, 0x05, 0x00, 0x00, 0x00, 0x00, 0x48, 0x85, 0xC0], mask="xxx????xxx"
    pub fn parse_pattern(pattern: &str) -> Result<(Vec<u8>, String)> {
        let mut bytes = Vec::new();
        let mut mask = String::new();

        for part in pattern.split_whitespace() {
            if part == "??" || part == "?" {
                bytes.push(0x00);
                mask.push('?');
            } else if part.len() == 2 {
                let byte = u8::from_str_radix(part, 16)
                    .map_err(|_| anyhow::anyhow!("Invalid hex byte: {}", part))?;
                bytes.push(byte);
                mask.push('x');
            } else {
                return Err(anyhow::anyhow!("Invalid pattern part: {}", part));
            }
        }

        Ok((bytes, mask))
    }

    /// Scan memory for a pattern
    pub async fn scan(&self, pid: u32, pattern: &str, start: u64, end: u64, options: Option<ScanOptions>) -> Result<Vec<u64>> {
        let (pattern_bytes, mask) = Self::parse_pattern(pattern)?;

        let req = ScanPatternRequest {
            pid,
            pattern: pattern.to_string(),
            start_address: start,
            end_address: end,
            options,
        };

        let resp = self.adapter.scan_pattern(req).await?;
        Ok(resp.addresses)
    }

    /// Scan a specific module for a pattern
    pub async fn scan_module(&self, pid: u32, module_name: &str, pattern: &str, options: Option<ScanOptions>) -> Result<Vec<u64>> {
        let req = ScanPatternModuleRequest {
            pid,
            module_name: module_name.to_string(),
            pattern: pattern.to_string(),
            options,
        };

        let resp = self.adapter.scan_pattern_module(req).await?;
        Ok(resp.addresses)
    }

    /// Find pattern using raw bytes and mask
    pub async fn find(&self, pid: u32, pattern: &[u8], mask: &str, start: u64, end: u64) -> Result<Vec<u64>> {
        let req = FindPatternRequest {
            pid,
            pattern: pattern.to_vec(),
            mask: mask.to_string(),
            start_address: start,
            end_address: end,
        };

        let resp = self.adapter.find_pattern(req).await?;
        Ok(resp.addresses)
    }

    /// Find first occurrence of pattern
    pub async fn find_first(&self, pid: u32, pattern: &str, start: u64, end: u64, options: Option<ScanOptions>) -> Result<Option<u64>> {
        let addresses = self.scan(pid, pattern, start, end, options).await?;
        Ok(addresses.into_iter().next())
    }

    /// Find all occurrences of pattern in a module
    pub async fn find_all_in_module(&self, pid: u32, module_name: &str, pattern: &str, options: Option<ScanOptions>) -> Result<Vec<u64>> {
        self.scan_module(pid, module_name, pattern, options).await
    }
}

/// AIDA64-style pattern helpers
pub mod ida {
    /// Create a pattern for a relative call/jmp (E8/E9 rel32)
    pub fn relative_call(offset: i32) -> String {
        let bytes = (offset as u32).to_le_bytes();
        format!("E8 {:02X} {:02X} {:02X} {:02X}", bytes[0], bytes[1], bytes[2], bytes[3])
    }

    /// Create a pattern for a RIP-relative addressing (48 8B 05 xx xx xx xx)
    pub fn rip_relative() -> &'static str {
        "48 8B 05 ?? ?? ?? ??"
    }

    /// Create a pattern for a VTable call (FF 15 xx xx xx xx)
    pub fn vtable_call() -> &'static str {
        "FF 15 ?? ?? ?? ??"
    }

    /// Create a pattern for a function prologue (48 89 5C 24 ?? 48 89 74 24 ?? 57 48 83 EC)
    pub fn function_prologue() -> &'static str {
        "48 89 5C 24 ?? 48 89 74 24 ?? 57 48 83 EC"
    }

    /// Create a pattern for a specific instruction sequence
    pub fn instructions(instructions: &[&str]) -> String {
        instructions.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pattern() {
        let (bytes, mask) = PatternScanner::parse_pattern("48 8B 05 ?? ?? ?? ?? 48 85 C0").unwrap();
        assert_eq!(bytes, vec![0x48, 0x8B, 0x05, 0x00, 0x00, 0x00, 0x00, 0x48, 0x85, 0xC0]);
        assert_eq!(mask, "xxx????xxx");
    }

    #[test]
    fn test_parse_pattern_single_wildcard() {
        let (bytes, mask) = PatternScanner::parse_pattern("48 8B ? 05").unwrap();
        assert_eq!(bytes, vec![0x48, 0x8B, 0x00, 0x05]);
        assert_eq!(mask, "xx?x");
    }

    #[test]
    fn test_ida_helpers() {
        assert_eq!(ida::relative_call(0x12345678), "E8 78 56 34 12");
        assert_eq!(ida::rip_relative(), "48 8B 05 ?? ?? ?? ??");
        assert_eq!(ida::vtable_call(), "FF 15 ?? ?? ?? ??");
    }
}