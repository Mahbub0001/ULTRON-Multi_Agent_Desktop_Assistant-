//! Authentication and authorization

use crate::config::{AuthConfig, CapabilityConfig};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,        // Client ID
    pub caps: Vec<Capability>, // Capabilities
    pub iat: i64,           // Issued at
    pub exp: i64,           // Expiration
    pub jti: String,        // JWT ID
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Capability {
    pub resource: String,
    pub actions: Vec<Action>,
    pub constraints: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Read,
    Write,
    Execute,
    Admin,
}

impl Action {
    pub fn implies(&self, other: Action) -> bool {
        match (self, other) {
            (Action::Admin, _) => true,
            (Action::Execute, Action::Read) => true,
            (Action::Write, Action::Read) => true,
            (a, b) => a == b,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub token_id: String,
    pub client_id: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub capabilities: Vec<Capability>,
    pub revoked: bool,
}

pub struct AuthManager {
    config: AuthConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    tokens: Arc<DashMap<String, TokenInfo>>,
    rng: SystemRandom,
}

impl AuthManager {
    pub fn new(config: AuthConfig) -> Result<Self> {
        let secret = config.jwt_secret.clone().unwrap_or_else(|| {
            let mut bytes = [0u8; 32];
            SystemRandom::new().fill(&mut bytes).unwrap();
            hex::encode(bytes)
        });
        
        let encoding_key = EncodingKey::from_secret(secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());
        
        Ok(Self {
            config,
            encoding_key,
            decoding_key,
            tokens: Arc::new(DashMap::new()),
            rng: SystemRandom::new(),
        })
    }
    
    pub fn issue_token(&self, client_id: String, capabilities: Vec<Capability>, ttl_seconds: u64) -> Result<String> {
        let now = Utc::now();
        let expires_at = now + Duration::seconds(ttl_seconds as i64);
        let token_id = Uuid::new_v4().to_string();
        
        let claims = Claims {
            sub: client_id.clone(),
            caps: capabilities.clone(),
            iat: now.timestamp(),
            exp: expires_at.timestamp(),
            jti: token_id.clone(),
        };
        
        let token = encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key)?;
        
        let token_info = TokenInfo {
            token_id: token_id.clone(),
            client_id,
            issued_at: now,
            expires_at,
            capabilities,
            revoked: false,
        };
        
        self.tokens.insert(token_id, token_info);
        
        info!("Issued token for client: {}", claims.sub);
        Ok(token)
    }
    
    pub fn validate_token(&self, token: &str, required_resource: &str, required_action: Action) -> Result<TokenData<Claims>> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        
        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)?;
        
        // Check if token is revoked
        if let Some(info) = self.tokens.get(&token_data.claims.jti) {
            if info.revoked {
                return Err(anyhow!("Token revoked"));
            }
        }
        
        // Check capabilities
        self.check_capabilities(&token_data.claims.caps, required_resource, required_action)?;
        
        Ok(token_data)
    }
    
    fn check_capabilities(&self, capabilities: &[Capability], resource: &str, action: Action) -> Result<()> {
        for cap in capabilities {
            if self.resource_matches(&cap.resource, resource) {
                for cap_action in &cap.actions {
                    if cap_action.implies(action) {
                        // Check constraints
                        if self.check_constraints(&cap.constraints)? {
                            return Ok(());
                        }
                    }
                }
            }
        }
        
        Err(anyhow!("Insufficient capabilities for {}:{}", resource, action as u8))
    }
    
    fn resource_matches(&self, pattern: &str, resource: &str) -> bool {
        if pattern == "*" || pattern == resource {
            return true;
        }
        
        // Wildcard matching (e.g., "input:*" matches "input:keyboard")
        if let Some(prefix) = pattern.strip_suffix("*") {
            return resource.starts_with(prefix);
        }
        
        false
    }
    
    fn check_constraints(&self, constraints: &HashMap<String, String>) -> Result<bool> {
        // Time-based constraints
        if let Some(allowed_hours) = constraints.get("allowed_hours") {
            let now = Utc::now();
            let hour = now.hour();
            let allowed: Vec<u32> = allowed_hours
                .split(',')
                .filter_map(|h| h.trim().parse().ok())
                .collect();
            if !allowed.contains(&hour) {
                return Ok(false);
            }
        }
        
        // Rate limiting constraints could be added here
        
        Ok(true)
    }
    
    pub fn revoke_token(&self, token_id: &str) -> Result<()> {
        if let Some(mut info) = self.tokens.get_mut(token_id) {
            info.revoked = true;
            info!("Revoked token: {}", token_id);
            Ok(())
        } else {
            Err(anyhow!("Token not found"))
        }
    }
    
    pub fn list_tokens(&self) -> Vec<TokenInfo> {
        self.tokens.iter().map(|e| e.value().clone()).collect()
    }
    
    pub fn cleanup_expired(&self) {
        let now = Utc::now();
        self.tokens.retain(|_, info| !info.revoked && info.expires_at > now);
    }
    
    pub fn is_admin_token(&self, token: &str) -> bool {
        self.config.admin_tokens.iter().any(|t| t == token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_action_implies() {
        assert!(Action::Admin.implies(Action::Read));
        assert!(Action::Admin.implies(Action::Write));
        assert!(Action::Admin.implies(Action::Execute));
        assert!(Action::Admin.implies(Action::Admin));
        
        assert!(Action::Execute.implies(Action::Read));
        assert!(!Action::Execute.implies(Action::Write));
        
        assert!(Action::Write.implies(Action::Read));
        assert!(!Action::Write.implies(Action::Execute));
        
        assert!(Action::Read.implies(Action::Read));
        assert!(!Action::Read.implies(Action::Write));
    }
    
    #[test]
    fn test_resource_matching() {
        let auth = AuthManager::new(AuthConfig::default()).unwrap();
        
        assert!(auth.resource_matches("*", "input:keyboard"));
        assert!(auth.resource_matches("input:*", "input:keyboard"));
        assert!(auth.resource_matches("input:*", "input:mouse"));
        assert!(auth.resource_matches("macro:copy", "macro:copy"));
        assert!(!auth.resource_matches("input:keyboard", "input:mouse"));
    }
}