//! Dataset definitions and snapshots (docs/15 §9).
//!
//! A dataset is a typed query result with columns, rows, and metadata.
//! Snapshots are materialized at report generation time for consistency.

use serde::{Deserialize, Serialize};

/// Dataset column type (docs/15 §9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatasetColumnType {
    /// Text column.
    Text,
    /// Integer column.
    Integer,
    /// Decimal column.
    Decimal,
    /// Money column.
    Money,
    /// Date column.
    Date,
    /// Datetime column.
    Datetime,
    /// Boolean column.
    Boolean,
    /// UUID column.
    Uuid,
    /// JSON column.
    Json,
}

/// A dataset column definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetColumn {
    /// Column name (machine name).
    pub name: String,
    /// Display label key (i18n).
    pub label_key: String,
    /// Column type.
    pub column_type: DatasetColumnType,
    /// Whether the column is nullable.
    pub nullable: bool,
    /// Whether the column is classified (restricted/confidential).
    pub classification: Option<String>,
    /// Format pattern (for dates/numbers).
    pub format_pattern: Option<String>,
}

/// A dataset query (docs/15 §9).
///
/// The query is a typed AST, not raw SQL. The query engine resolves it
/// against the schema/policy snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetQuery {
    /// Query type: "entity_list", "aggregate", "graph", "time_series".
    pub query_type: String,
    /// Target entity type alias.
    pub target_type: String,
    /// Filter expression (DSL, not raw SQL).
    pub filter: Option<String>,
    /// Group by columns.
    pub group_by: Vec<String>,
    /// Order by columns (column → asc/desc).
    pub order_by: Vec<(String, bool)>,
    /// Limit (max rows).
    pub limit: Option<u32>,
    /// Query parameters (parameter name → value).
    pub parameters: Vec<(String, String)>,
}

/// Dataset status (docs/15 §9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatasetStatus {
    /// Dataset definition is draft.
    Draft,
    /// Dataset is active.
    Active,
    /// Dataset snapshot is being materialized.
    Materializing,
    /// Dataset snapshot is ready.
    Ready,
    /// Dataset snapshot is stale (schema/policy changed).
    Stale,
    /// Dataset is retired.
    Retired,
}

/// A dataset definition (docs/15 §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dataset {
    /// Dataset ID.
    pub dataset_id: String,
    /// Dataset version.
    pub version: String,
    /// Dataset name.
    pub name: String,
    /// Dataset description key (i18n).
    pub description_key: String,
    /// Columns.
    pub columns: Vec<DatasetColumn>,
    /// Query definition.
    pub query: DatasetQuery,
    /// Status.
    pub status: DatasetStatus,
    /// Owner (team or user).
    pub owner: String,
    /// Whether the dataset requires snapshot consistency.
    pub requires_snapshot: bool,
}

/// A materialized dataset snapshot (docs/15 §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetSnapshot {
    /// Snapshot ID (UUID).
    pub snapshot_id: String,
    /// Dataset ID.
    pub dataset_id: String,
    /// Dataset version.
    pub dataset_version: String,
    /// Schema revision ID (pinned for consistency).
    pub schema_revision_id: String,
    /// Policy revision ID (pinned for consistency).
    pub policy_revision_id: String,
    /// Materialized at.
    pub materialized_at: chrono::DateTime<chrono::Utc>,
    /// Row count.
    pub row_count: u64,
    /// Content hash (for integrity verification).
    pub content_hash: String,
    /// Storage URL (where the snapshot data is stored).
    pub storage_url: String,
    /// Status.
    pub status: DatasetStatus,
}

impl DatasetSnapshot {
    /// Create a new materializing snapshot.
    pub fn new(dataset_id: &str, dataset_version: &str, schema_revision_id: &str, policy_revision_id: &str) -> Self {
        Self {
            snapshot_id: uuid::Uuid::now_v7().to_string(),
            dataset_id: dataset_id.to_string(),
            dataset_version: dataset_version.to_string(),
            schema_revision_id: schema_revision_id.to_string(),
            policy_revision_id: policy_revision_id.to_string(),
            materialized_at: chrono::Utc::now(),
            row_count: 0,
            content_hash: String::new(),
            storage_url: String::new(),
            status: DatasetStatus::Materializing,
        }
    }

    /// Mark snapshot as ready.
    pub fn mark_ready(&mut self, row_count: u64, content_hash: &str, storage_url: &str) {
        self.status = DatasetStatus::Ready;
        self.row_count = row_count;
        self.content_hash = content_hash.to_string();
        self.storage_url = storage_url.to_string();
        self.materialized_at = chrono::Utc::now();
    }

    /// Mark snapshot as stale.
    pub fn mark_stale(&mut self) {
        self.status = DatasetStatus::Stale;
    }

    /// Check if the snapshot is ready for use.
    pub fn is_ready(&self) -> bool {
        self.status == DatasetStatus::Ready
    }
}
