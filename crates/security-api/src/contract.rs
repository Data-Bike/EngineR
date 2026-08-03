//! Security API wire contract (docs/09 §3-5). Identical for local and remote
//! transport (ADR-008).

use serde::{Deserialize, Serialize};

/// Principal slice of the request context (docs/09 §3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityContextDto {
    pub request_id: String,
    pub tenant_id: uuid::Uuid,
    pub principal_id: uuid::Uuid,
    pub session_id: uuid::Uuid,
    pub auth_strength: String,
    pub groups_revision: i64,
    pub purpose: String,
    pub locale: String,
    pub time: chrono::DateTime<chrono::Utc>,
    pub client_service: String,
}

/// Action decision request (docs/09 §4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionDecisionRequest {
    pub context: SecurityContextDto,
    pub action: String,
    pub resource_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<uuid::Uuid>,
    #[serde(default)]
    pub parameters: serde_json::Map<String, serde_json::Value>,
}

/// Query decision request (docs/09 §5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryDecisionRequest {
    pub context: SecurityContextDto,
    pub action: String,
    pub root_type_ids: Vec<uuid::Uuid>,
    pub selected_predicate_ids: Vec<uuid::Uuid>,
    pub relation_predicate_ids: Vec<uuid::Uuid>,
    pub operations: Vec<String>,
    pub requested_page_size: u32,
    pub requested_graph_depth: u32,
    pub query_fingerprint: String,
}

/// Decision effect wire form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionEffectDto {
    Allow,
    AllowWithObligations,
    Deny,
    Indeterminate,
    Challenge,
}

impl From<enginer_security_domain::decision::DecisionEffect> for DecisionEffectDto {
    fn from(e: enginer_security_domain::decision::DecisionEffect) -> Self {
        use enginer_security_domain::decision::DecisionEffect as E;
        match e {
            E::Allow => Self::Allow,
            E::AllowWithObligations => Self::AllowWithObligations,
            E::Deny => Self::Deny,
            E::Indeterminate => Self::Indeterminate,
            E::Challenge => Self::Challenge,
        }
    }
}

/// Action decision response (docs/09 §4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionDecisionResponse {
    pub decision_id: uuid::Uuid,
    pub effect: DecisionEffectDto,
    pub policy_revision: i64,
    pub obligations: serde_json::Value,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Query decision response (docs/09 §5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryDecisionResponse {
    pub decision_id: uuid::Uuid,
    pub effect: DecisionEffectDto,
    pub policy_revision: i64,
    pub obligations: serde_json::Value,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}
