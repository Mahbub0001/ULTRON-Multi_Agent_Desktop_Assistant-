//! Chrome Session Management
//!
//! Manages CDP sessions for multiple targets.

use crate::adapters_proto::*;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// CDP Session wrapper
#[derive(Debug)]
pub struct CdpSession {
    session_id: String,
    target_id: String,
    connected: bool,
}

impl CdpSession {
    pub fn new(session_id: String, target_id: String) -> Self {
        Self {
            session_id,
            target_id,
            connected: true,
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
    }
}

/// Session manager for multiple CDP sessions
#[derive(Debug, Default)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Arc<CdpSession>>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new session
    pub async fn create_session(&self, session_id: String, target_id: String) -> Arc<CdpSession> {
        let session = Arc::new(CdpSession::new(session_id.clone(), target_id));
        self.sessions.write().await.insert(session_id, session.clone());
        info!(session_id = %session_id, target_id = %target_id, "Created CDP session");
        session
    }

    /// Get a session by ID
    pub async fn get_session(&self, session_id: &str) -> Option<Arc<CdpSession>> {
        self.sessions.read().await.get(session_id).cloned()
    }

    /// Get a session by target ID
    pub async fn get_session_by_target(&self, target_id: &str) -> Option<Arc<CdpSession>> {
        let sessions = self.sessions.read().await;
        sessions.values().find(|s| s.target_id == target_id).cloned()
    }

    /// Remove a session
    pub async fn remove_session(&self, session_id: &str) -> Option<Arc<CdpSession>> {
        let session = self.sessions.write().await.remove(session_id);
        if session.is_some() {
            info!(session_id = %session_id, "Removed CDP session");
        }
        session
    }

    /// List all sessions
    pub async fn list_sessions(&self) -> Vec<Arc<CdpSession>> {
        self.sessions.read().await.values().cloned().collect()
    }

    /// Clean up disconnected sessions
    pub async fn cleanup(&self) {
        let mut sessions = self.sessions.write().await;
        sessions.retain(|_, s| s.is_connected());
    }
}

/// High-level Chrome session API
pub struct ChromeSessionApi {
    manager: Arc<SessionManager>,
    // In real implementation, this would hold the CDP connection
}

impl ChromeSessionApi {
    pub fn new(manager: Arc<SessionManager>) -> Self {
        Self { manager }
    }

    /// Get or create a session for a target
    pub async fn get_or_create_session(&self, target_id: &str) -> Result<Arc<CdpSession>> {
        if let Some(session) = self.manager.get_session_by_target(target_id).await {
            return Ok(session);
        }

        let session_id = format!("session_{}", uuid::Uuid::new_v4());
        let session = self.manager.create_session(session_id, target_id.to_string()).await;
        Ok(session)
    }

    /// Close a session
    pub async fn close_session(&self, session_id: &str) -> Result<()> {
        self.manager.remove_session(session_id).await;
        Ok(())
    }

    /// Close all sessions for a target
    pub async fn close_target_sessions(&self, target_id: &str) -> Result<()> {
        let sessions = self.manager.list_sessions().await;
        for session in sessions {
            if session.target_id() == target_id {
                self.manager.remove_session(session.session_id()).await;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_manager() {
        let manager = SessionManager::new();

        let session = manager.create_session("sess_1".to_string(), "target_1".to_string()).await;
        assert_eq!(session.session_id(), "sess_1");
        assert_eq!(session.target_id(), "target_1");

        let found = manager.get_session("sess_1").await;
        assert!(found.is_some());

        let found_by_target = manager.get_session_by_target("target_1").await;
        assert!(found_by_target.is_some());

        manager.remove_session("sess_1").await;
        assert!(manager.get_session("sess_1").await.is_none());
    }

    #[tokio::test]
    async fn test_chrome_session_api() {
        let manager = Arc::new(SessionManager::new());
        let api = ChromeSessionApi::new(manager);

        let session = api.get_or_create_session("target_1").await.unwrap();
        assert_eq!(session.target_id(), "target_1");

        // Getting again should return same session
        let session2 = api.get_or_create_session("target_1").await.unwrap();
        assert_eq!(session.session_id(), session2.session_id());
    }
}