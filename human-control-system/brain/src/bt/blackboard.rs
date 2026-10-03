//! Blackboard implementation for behavior tree shared state

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Thread-safe blackboard for concurrent access
#[derive(Debug, Clone, Default)]
pub struct Blackboard {
    data: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    parent: Option<Arc<Blackboard>>,
}

impl Blackboard {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_parent(parent: Blackboard) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            parent: Some(Arc::new(parent)),
        }
    }
    
    pub fn get(&self, key: &str) -> Option<serde_json::Value> {
        self.data.read().get(key).cloned()
            .or_else(|| self.parent.as_ref()?.get(key))
    }
    
    pub fn get_string(&self, key: &str) -> Option<String> {
        self.get(key).and_then(|v| v.as_str().map(|s| s.to_string()))
    }
    
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|v| v.as_bool())
    }
    
    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.as_i64())
    }
    
    pub fn get_float(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(|v| v.as_f64())
    }
    
    pub fn get_array(&self, key: &str) -> Option<Vec<serde_json::Value>> {
        self.get(key).and_then(|v| v.as_array().cloned())
    }
    
    pub fn get_object(&self, key: &str) -> Option<HashMap<String, serde_json::Value>> {
        self.get(key).and_then(|v| v.as_object().map(|m| m.clone().into_iter().collect()))
    }
    
    pub fn set(&self, key: impl Into<String>, value: impl Into<serde_json::Value>) {
        self.data.write().insert(key.into(), value.into());
    }
    
    pub fn has(&self, key: &str) -> bool {
        self.data.read().contains_key(key) || self.parent.as_ref().map_or(false, |p| p.has(key))
    }
    
    pub fn remove(&self, key: &str) -> Option<serde_json::Value> {
        self.data.write().remove(key)
    }
    
    pub fn clear(&self) {
        self.data.write().clear();
    }
    
    pub fn keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.data.read().keys().cloned().collect();
        if let Some(parent) = &self.parent {
            for key in parent.keys() {
                if !keys.contains(&key) {
                    keys.push(key);
                }
            }
        }
        keys
    }
    
    pub fn merge(&self, other: &Blackboard) {
        let mut data = self.data.write();
        let other_data = other.data.read();
        for (k, v) in other_data.iter() {
            data.insert(k.clone(), v.clone());
        }
    }
    
    pub fn snapshot(&self) -> HashMap<String, serde_json::Value> {
        self.data.read().clone()
    }
    
    pub fn len(&self) -> usize {
        self.data.read().len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.data.read().is_empty()
    }
}

/// Serializable blackboard for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableBlackboard {
    pub data: HashMap<String, serde_json::Value>,
}

impl From<&Blackboard> for SerializableBlackboard {
    fn from(bb: &Blackboard) -> Self {
        Self {
            data: bb.snapshot(),
        }
    }
}

impl From<SerializableBlackboard> for Blackboard {
    fn from(sbb: SerializableBlackboard) -> Self {
        Self {
            data: Arc::new(RwLock::new(sbb.data)),
            parent: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_operations() {
        let bb = Blackboard::new();
        bb.set("key1", "value1");
        bb.set("key2", 42);
        bb.set("key3", true);
        
        assert_eq!(bb.get_string("key1"), Some("value1".to_string()));
        assert_eq!(bb.get_int("key2"), Some(42));
        assert_eq!(bb.get_bool("key3"), Some(true));
        assert!(bb.has("key1"));
        assert!(!bb.has("nonexistent"));
    }
    
    #[test]
    fn test_parent_child() {
        let parent = Blackboard::new();
        parent.set("parent_key", "parent_value");
        
        let child = Blackboard::with_parent(parent);
        child.set("child_key", "child_value");
        
        assert_eq!(child.get_string("parent_key"), Some("parent_value".to_string()));
        assert_eq!(child.get_string("child_key"), Some("child_value".to_string()));
        assert!(child.has("parent_key"));
    }
    
    #[test]
    fn test_remove() {
        let bb = Blackboard::new();
        bb.set("key", "value");
        assert!(bb.has("key"));
        
        let removed = bb.remove("key");
        assert_eq!(removed, Some(serde_json::Value::String("value".to_string())));
        assert!(!bb.has("key"));
    }
    
    #[test]
    fn test_snapshot() {
        let bb = Blackboard::new();
        bb.set("key1", "value1");
        bb.set("key2", 42);
        
        let snapshot = bb.snapshot();
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot.get("key1"), Some(&serde_json::Value::String("value1".to_string())));
    }
    
    #[test]
    fn test_serialization() {
        let bb = Blackboard::new();
        bb.set("key1", "value1");
        bb.set("key2", 42);
        
        let serializable: SerializableBlackboard = (&bb).into();
        let restored = Blackboard::from(serializable);
        
        assert_eq!(restored.get_string("key1"), Some("value1".to_string()));
        assert_eq!(restored.get_int("key2"), Some(42));
    }
}
impl Serialize for Blackboard {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializableBlackboard::from(self).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for Blackboard {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(SerializableBlackboard::deserialize(deserializer)?.into())
    }
}
