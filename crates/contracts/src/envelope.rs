//! API envelope (docs/06 §2).

use serde::{Deserialize, Serialize};

/// Success envelope: `{ data, meta, links }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEnvelope<T> {
    pub data: T,
    pub meta: ApiMeta,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub links: Option<ApiLinks>,
}

/// Response metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApiMeta {
    pub request_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_revision: Option<String>,
}

/// Pagination / navigation links.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApiLinks {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev: Option<String>,
}
