//! Query AST (docs/07 §2).
//!
//! The AST references stable type/predicate IDs only — never table or column
//! names. The compiler resolves them from the trusted catalog. User literals
//! are carried as `serde_json::Value` and always become bind parameters.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A field reference: either a predicate id (data field) or a system field
/// (`public_id`, `entity_id`, `created_at`, `status`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldRef {
    /// A data field backed by a statement predicate.
    Predicate { predicate_id: uuid::Uuid },
    /// A system field on the entity itself.
    System { system: SystemField },
}

/// System fields queryable without a statement join.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemField {
    PublicId,
    EntityId,
    CreatedAt,
    Status,
    Revision,
}

/// Comparison operator (docs/07 §3). Regex/script/SQL fragment are forbidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    Eq,
    Ne,
    In,
    IsNull,
    IsNotNull,
    Lt,
    Lte,
    Gt,
    Gte,
    Between,
    Contains,
    Prefix,
    FullText,
    ValidAt,
    RelationExists,
    RelationNotExists,
}

/// A where expression tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhereExpr {
    /// Leaf comparison: field op value(s).
    Comparison {
        field: FieldRef,
        op: Operator,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        value: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        values: Option<Vec<serde_json::Value>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        valid_at: Option<chrono::DateTime<chrono::Utc>>,
    },
    /// Relation existence check (edge traversal).
    Relation {
        predicate_id: uuid::Uuid,
        direction: RelationDirection,
        target_type_id: Option<uuid::Uuid>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filter: Option<Box<WhereExpr>>,
    },
    And(Vec<WhereExpr>),
    Or(Vec<WhereExpr>),
    Not(Box<WhereExpr>),
}

/// Relation direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationDirection {
    Outgoing,
    Incoming,
    Both,
}

/// A select field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectField {
    pub field: FieldRef,
    pub alias: String,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    Asc,
    Desc,
}

/// A sort key. The compiler always appends `public_id` as a tie-breaker
/// (docs/07 §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SortKey {
    pub field: FieldRef,
    pub direction: SortDirection,
}

/// Keyset page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeysetPage {
    pub size: u32,
    /// Opaque signed cursor from a previous response, or `None` for the first
    /// page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}

/// The root query AST (docs/07 §2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryAst {
    pub from: FromClause,
    pub select: Vec<SelectField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub where_expr: Option<WhereExpr>,
    #[serde(default)]
    pub sort: Vec<SortKey>,
    pub page: KeysetPage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub as_of: Option<chrono::DateTime<chrono::Utc>>,
}

/// The `from` clause: root type + alias.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FromClause {
    pub type_id: uuid::Uuid,
    pub alias: String,
}

/// Parse-time query error (docs/07 §13).
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum QueryError {
    #[error("syntax_invalid: {message}")]
    SyntaxInvalid { message: String },
    #[error("unknown_type: {type_id}")]
    UnknownType { type_id: uuid::Uuid },
    #[error("unknown_predicate: {predicate_id}")]
    UnknownPredicate { predicate_id: uuid::Uuid },
    #[error("operator_not_supported: {operator}")]
    OperatorNotSupported { operator: String },
    #[error("field_forbidden: {message}")]
    FieldForbidden { message: String },
    #[error("too_complex: {message}")]
    TooComplex { message: String },
    #[error("requires_batch")]
    RequiresBatch,
    #[error("cursor_invalid: {message}")]
    CursorInvalid { message: String },
    #[error("cursor_stale")]
    CursorStale,
    #[error("projection_lag")]
    ProjectionLag,
    #[error("timeout")]
    Timeout,
}

impl QueryAst {
    /// Enforce syntactic limits (docs/06 §16): max select fields, max sort
    /// keys, max where depth, max page size.
    pub fn enforce_syntactic_limits(
        &self,
        max_select: usize,
        max_sort: usize,
        max_page: u32,
    ) -> Result<(), QueryError> {
        if self.select.len() > max_select {
            return Err(QueryError::TooComplex {
                message: format!("too many select fields (max {max_select})"),
            });
        }
        if self.sort.len() > max_sort {
            return Err(QueryError::TooComplex {
                message: format!("too many sort keys (max {max_sort})"),
            });
        }
        if self.page.size > max_page {
            return Err(QueryError::TooComplex {
                message: format!("page size too large (max {max_page})"),
            });
        }
        if self.page.size == 0 {
            return Err(QueryError::SyntaxInvalid {
                message: "page size must be > 0".to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ast_roundtrips_json() {
        let ast = QueryAst {
            from: FromClause {
                type_id: uuid::Uuid::now_v7(),
                alias: "e".to_string(),
            },
            select: vec![SelectField {
                field: FieldRef::System {
                    system: SystemField::PublicId,
                },
                alias: "id".to_string(),
            }],
            where_expr: Some(WhereExpr::Comparison {
                field: FieldRef::System {
                    system: SystemField::Status,
                },
                op: Operator::Eq,
                value: Some(serde_json::json!("active")),
                values: None,
                valid_at: None,
            }),
            sort: vec![SortKey {
                field: FieldRef::System {
                    system: SystemField::PublicId,
                },
                direction: SortDirection::Asc,
            }],
            page: KeysetPage {
                size: 50,
                after: None,
            },
            as_of: None,
        };
        let json = serde_json::to_string(&ast).unwrap();
        let back: QueryAst = serde_json::from_str(&json).unwrap();
        assert_eq!(ast, back);
    }

    #[test]
    fn limits_reject_oversized() {
        let ast = QueryAst {
            from: FromClause {
                type_id: uuid::Uuid::now_v7(),
                alias: "e".to_string(),
            },
            select: (0..10)
                .map(|i| SelectField {
                    field: FieldRef::System {
                        system: SystemField::PublicId,
                    },
                    alias: format!("f{i}"),
                })
                .collect(),
            where_expr: None,
            sort: vec![],
            page: KeysetPage {
                size: 500,
                after: None,
            },
            as_of: None,
        };
        assert!(ast.enforce_syntactic_limits(5, 5, 100).is_err());
    }
}
