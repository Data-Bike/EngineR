//! Typed API configuration (docs/23 §4). No secrets in the file; the DB URL
//! comes from a secret ref / environment.

use serde::{Deserialize, Serialize};

/// API configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub http: HttpConfig,
    pub database: DatabaseConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    pub public_origin: String,
    pub request_timeout_ms: u64,
    pub max_json_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Secret ref or direct URL (env override preferred).
    pub url_secret_ref: String,
    pub max_connections: u32,
    pub acquire_timeout_ms: u64,
    pub statement_timeout_ms: u64,
    pub db_executor_queue: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub policy_cache_ttl_seconds: u64,
    pub fail_closed: bool,
    pub internal_mtls_identity: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            http: HttpConfig {
                public_origin: "http://localhost:8000".to_string(),
                request_timeout_ms: 5000,
                max_json_bytes: 1_048_576,
            },
            database: DatabaseConfig {
                url_secret_ref: "secret://enginer/db".to_string(),
                max_connections: 40,
                acquire_timeout_ms: 500,
                statement_timeout_ms: 4000,
                db_executor_queue: 200,
            },
            security: SecurityConfig {
                policy_cache_ttl_seconds: 30,
                fail_closed: true,
                internal_mtls_identity: "enginer-api".to_string(),
            },
        }
    }
}

impl ApiConfig {
    /// Load config from defaults → file → environment (docs/05 §11).
    ///
    /// `ENGINER_DATABASE_URL` overrides the DB URL secret ref (local dev / tests).
    pub fn load() -> Result<Self, ConfigError> {
        let mut cfg = ApiConfig::default();
        if let Ok(path) = std::env::var("ENGINER_CONFIG") {
            let text = std::fs::read_to_string(&path).map_err(|e| ConfigError::Read {
                path: path.clone(),
                message: e.to_string(),
            })?;
            cfg = toml::from_str(&text).map_err(|e| ConfigError::Parse(e.to_string()))?;
        }
        if let Ok(url) = std::env::var("ENGINER_DATABASE_URL") {
            cfg.database.url_secret_ref = url;
        }
        Ok(cfg)
    }
}

/// Config error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error("read {path}: {message}")]
    Read { path: String, message: String },
    #[error("parse: {0}")]
    Parse(String),
}
