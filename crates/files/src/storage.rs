//! Object storage abstraction (docs/15 §1, §14).
//!
//! Pluggable storage backend (S3, Azure Blob, local FS). Objects are
//! immutable, content-addressed, with classification-based TTL/legal hold.

use serde::{Deserialize, Serialize};

/// Storage class (docs/15 §14).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageClass {
    /// Hot storage (frequently accessed).
    Hot,
    /// Cool storage (infrequently accessed).
    Cool,
    /// Archive storage (rarely accessed).
    Archive,
}

impl StorageClass {
    /// Parse from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "hot" => Some(Self::Hot),
            "cool" => Some(Self::Cool),
            "archive" => Some(Self::Archive),
            _ => None,
        }
    }
}

/// Storage configuration (docs/15 §14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Backend type: "s3", "azure_blob", "local_fs".
    pub backend: String,
    /// Bucket/container name.
    pub container: String,
    /// Default storage class.
    pub default_storage_class: StorageClass,
    /// Default TTL in days (None = indefinite).
    pub default_ttl_days: Option<u32>,
    /// Whether encryption is enabled.
    pub encryption_enabled: bool,
    /// Whether to use content-addressed storage (hash-based key).
    pub content_addressed: bool,
}

/// Object storage URL (content-addressed or path-based).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StorageUrl {
    /// Backend-specific URL (e.g. "s3://bucket/key").
    pub url: String,
    /// Object ID.
    pub object_id: String,
}

/// Object metadata (docs/15 §14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectMetadata {
    /// Object ID (UUID).
    pub object_id: String,
    /// Tenant ID.
    pub tenant_id: String,
    /// Content hash (blake2b or sha256).
    pub content_hash: String,
    /// Hash algorithm.
    pub hash_algorithm: String,
    /// Size in bytes.
    pub size: u64,
    /// Content type (MIME).
    pub content_type: String,
    /// Original filename.
    pub original_filename: String,
    /// Storage class.
    pub storage_class: StorageClass,
    /// Storage URL.
    pub storage_url: StorageUrl,
    /// Classification level: "public", "restricted", "confidential".
    pub classification: String,
    /// Created at.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// TTL expiry (None = indefinite).
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Whether legal hold is applied (suspends expiry).
    pub legal_hold: bool,
    /// Whether the object is encrypted.
    pub encrypted: bool,
    /// User metadata (key → value).
    pub user_metadata: Vec<(String, String)>,
}

impl ObjectMetadata {
    /// Create new object metadata.
    pub fn new(
        tenant_id: &str,
        content_hash: &str,
        hash_algorithm: &str,
        size: u64,
        content_type: &str,
        original_filename: &str,
        storage_class: StorageClass,
        storage_url: StorageUrl,
        classification: &str,
    ) -> Self {
        let object_id = uuid::Uuid::now_v7().to_string();
        Self {
            object_id,
            tenant_id: tenant_id.to_string(),
            content_hash: content_hash.to_string(),
            hash_algorithm: hash_algorithm.to_string(),
            size,
            content_type: content_type.to_string(),
            original_filename: original_filename.to_string(),
            storage_class,
            storage_url,
            classification: classification.to_string(),
            created_at: chrono::Utc::now(),
            expires_at: None,
            legal_hold: false,
            encrypted: false,
            user_metadata: Vec::new(),
        }
    }

    /// Set TTL expiry.
    pub fn with_ttl(mut self, days: u32) -> Self {
        self.expires_at = Some(self.created_at + chrono::Duration::days(days as i64));
        self
    }

    /// Apply legal hold.
    pub fn apply_legal_hold(&mut self) {
        self.legal_hold = true;
    }

    /// Release legal hold.
    pub fn release_legal_hold(&mut self) {
        self.legal_hold = false;
    }

    /// Check if the object is expired.
    pub fn is_expired(&self) -> bool {
        !self.legal_hold
            && self.expires_at
                .map(|exp| chrono::Utc::now() > exp)
                .unwrap_or(false)
    }

    /// Add user metadata.
    pub fn add_metadata(&mut self, key: &str, value: &str) {
        self.user_metadata.push((key.to_string(), value.to_string()));
    }
}

/// Object storage trait (docs/15 §1, §14).
///
/// Pluggable backend abstraction. Implementations: S3, Azure Blob, local FS.
pub trait ObjectStorage: Send + Sync {
    /// Store an object.
    fn store(&self, data: &[u8], metadata: &ObjectMetadata) -> Result<StorageUrl, String>;

    /// Retrieve an object (returns data + metadata).
    fn retrieve(&self, url: &StorageUrl) -> Result<(Vec<u8>, ObjectMetadata), String>;

    /// Delete an object (cryptographic delete if configured).
    fn delete(&self, url: &StorageUrl) -> Result<(), String>;

    /// Check if an object exists.
    fn exists(&self, url: &StorageUrl) -> Result<bool, String>;

    /// Get object metadata without retrieving data.
    fn head(&self, url: &StorageUrl) -> Result<ObjectMetadata, String>;
}
