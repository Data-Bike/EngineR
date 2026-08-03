//! Risk criterion DSL AST (docs/14 §8).
//!
//! Typed expression AST — no arbitrary code. Supports:
//! - compare/range/existence/count
//! - temporal age/duration
//! - string exact/normalized set
//! - graph relation exists/path with bounded depth
//! - source freshness/trust/conflict
//! - arithmetic decimal
//! - boolean all/any/not
//! - lookup in versioned dictionary
//! - previous assessment change

use serde::{Deserialize, Serialize};

/// A typed value in the DSL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Value {
    /// Decimal value (canonical string for precision).
    Decimal { value: String },
    /// Integer value.
    Integer { value: i64 },
    /// Text value (NFC normalized).
    Text { value: String },
    /// Boolean value.
    Boolean { value: bool },
    /// Date value (YYYY-MM-DD).
    Date { value: String },
    /// Duration in days.
    DurationDays { value: i64 },
    /// Reference to a predicate value (resolved at evaluation time).
    PredicateRef { predicate_alias: String },
    /// Reference to a source observation field.
    ObservationRef { source_field: String },
    /// Null / no value.
    Null,
}

/// Comparison operators (docs/14 §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOp {
    /// Equal.
    Eq,
    /// Not equal.
    Ne,
    /// Less than.
    Lt,
    /// Less than or equal.
    Le,
    /// Greater than.
    Gt,
    /// Greater than or equal.
    Ge,
    /// String prefix match.
    Prefix,
    /// String contains.
    Contains,
    /// String normalized equality (NFC + casefold).
    NormalizedEq,
}

/// Binary arithmetic operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

/// Logical operators (boolean all/any/not, docs/14 §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogicalOp {
    /// All children must be true.
    All,
    /// Any child must be true.
    Any,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnaryOp {
    /// Logical not.
    Not,
    /// Existence check (value is not null).
    Exists,
    /// Count of values (for multi-valued predicates).
    Count,
}

/// Expression kind — the AST node type (docs/14 §8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExprKind {
    /// Literal value.
    Literal { value: Value },
    /// Comparison: left op right.
    Compare {
        op: ComparisonOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// Range check: value in [min, max].
    Range {
        value: Box<Expr>,
        min: Box<Expr>,
        max: Box<Expr>,
        inclusive_min: bool,
        inclusive_max: bool,
    },
    /// Existence check (predicate has at least one value).
    Existence { predicate_alias: String },
    /// Count of values for a predicate.
    Count { predicate_alias: String },
    /// Temporal age: age of a date value in days.
    Age {
        date_expr: Box<Expr>,
        as_of: Box<Expr>,
    },
    /// Duration between two dates.
    Duration {
        from: Box<Expr>,
        to: Box<Expr>,
    },
    /// String exact match.
    StringExact {
        predicate_alias: String,
        expected: String,
    },
    /// String normalized set membership (NFC + casefold).
    StringNormalizedSet {
        predicate_alias: String,
        expected_set: Vec<String>,
    },
    /// Graph: relation exists between target and another entity.
    RelationExists {
        predicate_alias: String,
        target_type: Option<String>,
    },
    /// Graph: path exists with bounded depth.
    RelationPath {
        predicate_alias: String,
        max_depth: u32,
        target_type: Option<String>,
    },
    /// Source freshness: observation age in days.
    SourceFreshness {
        source_connector_type_id: String,
        max_age_days: i64,
    },
    /// Source trust rank check.
    SourceTrust {
        min_trust_rank: u8,
    },
    /// Source conflict: multiple sources disagree.
    SourceConflict { predicate_alias: String },
    /// Arithmetic binary operation.
    Arithmetic {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// Logical: all/any children.
    Logical {
        op: LogicalOp,
        children: Vec<Expr>,
    },
    /// Unary: not/exists/count.
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    /// Lookup in versioned dictionary.
    DictionaryLookup {
        dictionary_id: String,
        dictionary_version: String,
        key_expr: Box<Expr>,
    },
    /// Previous assessment change: compare with previous assessment.
    PreviousAssessmentChange {
        criterion_id: String,
    },
}

/// A typed expression in the criterion DSL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Expr {
    #[serde(flatten)]
    pub kind: ExprKind,
}

/// Convenience alias.
pub type AstExpr = Expr;

impl Expr {
    /// Create a literal expression.
    pub fn literal(value: Value) -> Self {
        Self {
            kind: ExprKind::Literal { value },
        }
    }

    /// Create a comparison expression.
    pub fn compare(op: ComparisonOp, left: Expr, right: Expr) -> Self {
        Self {
            kind: ExprKind::Compare {
                op,
                left: Box::new(left),
                right: Box::new(right),
            },
        }
    }

    /// Create a logical expression.
    pub fn logical(op: LogicalOp, children: Vec<Expr>) -> Self {
        Self {
            kind: ExprKind::Logical { op, children },
        }
    }

    /// Create a unary expression.
    pub fn unary(op: UnaryOp, operand: Expr) -> Self {
        Self {
            kind: ExprKind::Unary {
                op,
                operand: Box::new(operand),
            },
        }
    }

    /// Create an existence check.
    pub fn exists(predicate_alias: &str) -> Self {
        Self {
            kind: ExprKind::Existence {
                predicate_alias: predicate_alias.to_string(),
            },
        }
    }

    /// Create a range check.
    pub fn range(
        value: Expr,
        min: Expr,
        max: Expr,
        inclusive_min: bool,
        inclusive_max: bool,
    ) -> Self {
        Self {
            kind: ExprKind::Range {
                value: Box::new(value),
                min: Box::new(min),
                max: Box::new(max),
                inclusive_min,
                inclusive_max,
            },
        }
    }
}

/// Evaluate an expression against a context (predicate values + observations).
///
/// The evaluator is pure: it takes an expression and a context and returns
/// a value. No side effects, no I/O (docs/14 §8).
pub fn evaluate(
    expr: &Expr,
    ctx: &EvaluationContext,
) -> Result<Value, crate::error::RiskError> {
    match &expr.kind {
        ExprKind::Literal { value } => Ok(value.clone()),
        ExprKind::Compare { op, left, right } => {
            let l = evaluate(left, ctx)?;
            let r = evaluate(right, ctx)?;
            evaluate_compare(*op, &l, &r)
        }
        ExprKind::Range {
            value,
            min,
            max,
            inclusive_min,
            inclusive_max,
        } => {
            let v = evaluate(value, ctx)?;
            let lo = evaluate(min, ctx)?;
            let hi = evaluate(max, ctx)?;
            evaluate_range(&v, &lo, &hi, *inclusive_min, *inclusive_max)
        }
        ExprKind::Existence { predicate_alias } => {
            let exists = ctx.predicate_values.contains_key(predicate_alias);
            Ok(Value::Boolean { value: exists })
        }
        ExprKind::Count { predicate_alias } => {
            let count = ctx
                .predicate_values
                .get(predicate_alias)
                .map(|v| v.len() as i64)
                .unwrap_or(0);
            Ok(Value::Integer { value: count })
        }
        ExprKind::Age { date_expr, as_of } => {
            let date = evaluate(date_expr, ctx)?;
            let ref_date = evaluate(as_of, ctx)?;
            evaluate_age(&date, &ref_date)
        }
        ExprKind::Duration { from, to } => {
            let f = evaluate(from, ctx)?;
            let t = evaluate(to, ctx)?;
            evaluate_duration(&f, &t)
        }
        ExprKind::StringExact {
            predicate_alias,
            expected,
        } => {
            let actual = ctx.predicate_values.get(predicate_alias)
                .and_then(|vals| vals.first())
                .and_then(|v| match v {
                    Value::Text { value } => Some(value.clone()),
                    _ => None,
                });
            Ok(Value::Boolean {
                value: actual.as_deref() == Some(expected.as_str()),
            })
        }
        ExprKind::StringNormalizedSet {
            predicate_alias,
            expected_set,
        } => {
            let actual = ctx.predicate_values.get(predicate_alias)
                .and_then(|vals| vals.first())
                .and_then(|v| match v {
                    Value::Text { value } => Some(normalize(value)),
                    _ => None,
                });
            let normalized_set: Vec<String> =
                expected_set.iter().map(|s| normalize(s)).collect();
            Ok(Value::Boolean {
                value: actual
                    .map(|a| normalized_set.contains(&a))
                    .unwrap_or(false),
            })
        }
        ExprKind::RelationExists { .. } => {
            // Graph traversal requires entity graph access — placeholder
            Ok(Value::Boolean { value: false })
        }
        ExprKind::RelationPath { .. } => {
            // Graph traversal requires entity graph access — placeholder
            Ok(Value::Boolean { value: false })
        }
        ExprKind::SourceFreshness {
            source_connector_type_id,
            max_age_days,
        } => {
            let fresh = ctx
                .observations
                .iter()
                .any(|obs| {
                    obs.connector_type_id == *source_connector_type_id
                        && obs.age_days <= *max_age_days
                });
            Ok(Value::Boolean { value: fresh })
        }
        ExprKind::SourceTrust { min_trust_rank } => {
            let trusted = ctx
                .observations
                .iter()
                .any(|obs| obs.trust_rank >= *min_trust_rank);
            Ok(Value::Boolean { value: trusted })
        }
        ExprKind::SourceConflict { .. } => {
            // Conflict detection requires multi-source comparison — placeholder
            Ok(Value::Boolean { value: false })
        }
        ExprKind::Arithmetic { op, left, right } => {
            let l = evaluate(left, ctx)?;
            let r = evaluate(right, ctx)?;
            evaluate_arithmetic(*op, &l, &r)
        }
        ExprKind::Logical { op, children } => {
            let results: Vec<bool> = children
                .iter()
                .map(|c| evaluate(c, ctx))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(|v| match v {
                    Value::Boolean { value } => value,
                    _ => false,
                })
                .collect();
            let result = match op {
                LogicalOp::All => results.iter().all(|&b| b),
                LogicalOp::Any => results.iter().any(|&b| b),
            };
            Ok(Value::Boolean { value: result })
        }
        ExprKind::Unary { op, operand } => {
            let v = evaluate(operand, ctx)?;
            match op {
                UnaryOp::Not => match v {
                    Value::Boolean { value } => Ok(Value::Boolean { value: !value }),
                    _ => Err(crate::error::RiskError::TypeMismatch {
                        expected: "boolean".to_string(),
                        got: type_name(&v).to_string(),
                    }),
                },
                UnaryOp::Exists => Ok(Value::Boolean {
                    value: !matches!(v, Value::Null),
                }),
                UnaryOp::Count => match v {
                    Value::Text { value } => Ok(Value::Integer {
                        value: value.len() as i64,
                    }),
                    _ => Err(crate::error::RiskError::TypeMismatch {
                        expected: "text".to_string(),
                        got: type_name(&v).to_string(),
                    }),
                },
            }
        }
        ExprKind::DictionaryLookup {
            dictionary_id,
            dictionary_version,
            key_expr,
        } => {
            let key = evaluate(key_expr, ctx)?;
            // Dictionary lookup requires dictionary access — placeholder
            let _ = (dictionary_id, dictionary_version);
            Ok(Value::Boolean {
                value: matches!(key, Value::Text { .. }),
            })
        }
        ExprKind::PreviousAssessmentChange { criterion_id } => {
            let changed = ctx
                .previous_factors
                .iter()
                .any(|f| f.criterion_id == *criterion_id);
            Ok(Value::Boolean { value: changed })
        }
    }
}

/// Context for expression evaluation.
#[derive(Debug, Clone, Default)]
pub struct EvaluationContext {
    /// Predicate values keyed by alias.
    pub predicate_values: std::collections::HashMap<String, Vec<Value>>,
    /// Source observations.
    pub observations: Vec<ObservationContext>,
    /// Previous assessment factors (for change detection).
    pub previous_factors: Vec<PreviousFactorContext>,
}

/// Observation context for freshness/trust evaluation.
#[derive(Debug, Clone)]
pub struct ObservationContext {
    pub connector_type_id: String,
    pub age_days: i64,
    pub trust_rank: u8,
}

/// Previous factor context for change detection.
#[derive(Debug, Clone)]
pub struct PreviousFactorContext {
    pub criterion_id: String,
    pub severity: String,
}

// --- Helper functions ---

fn normalize(s: &str) -> String {
    // Basic normalization: lowercase + trim
    // Full implementation would use unicode-normalization crate
    s.trim().to_lowercase()
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Decimal { .. } => "decimal",
        Value::Integer { .. } => "integer",
        Value::Text { .. } => "text",
        Value::Boolean { .. } => "boolean",
        Value::Date { .. } => "date",
        Value::DurationDays { .. } => "duration_days",
        Value::PredicateRef { .. } => "predicate_ref",
        Value::ObservationRef { .. } => "observation_ref",
        Value::Null => "null",
    }
}

fn evaluate_compare(
    op: ComparisonOp,
    left: &Value,
    right: &Value,
) -> Result<Value, crate::error::RiskError> {
    let result = match (left, right) {
        (Value::Integer { value: l }, Value::Integer { value: r }) => match op {
            ComparisonOp::Eq => l == r,
            ComparisonOp::Ne => l != r,
            ComparisonOp::Lt => l < r,
            ComparisonOp::Le => l <= r,
            ComparisonOp::Gt => l > r,
            ComparisonOp::Ge => l >= r,
            _ => return Err(crate::error::RiskError::TypeMismatch {
                expected: "comparable".to_string(),
                got: "incompatible_op".to_string(),
            }),
        },
        (Value::Text { value: l }, Value::Text { value: r }) => match op {
            ComparisonOp::Eq => l == r,
            ComparisonOp::Ne => l != r,
            ComparisonOp::Prefix => r.starts_with(l.as_str()) || l.starts_with(r.as_str()),
            ComparisonOp::Contains => l.contains(r.as_str()) || r.contains(l.as_str()),
            ComparisonOp::NormalizedEq => normalize(l) == normalize(r),
            _ => return Err(crate::error::RiskError::TypeMismatch {
                expected: "text-comparable".to_string(),
                got: "incompatible_op".to_string(),
            }),
        },
        (Value::Decimal { value: l }, Value::Decimal { value: r }) => {
            let ld: f64 = l.parse().unwrap_or(0.0);
            let rd: f64 = r.parse().unwrap_or(0.0);
            match op {
                ComparisonOp::Eq => (ld - rd).abs() < f64::EPSILON,
                ComparisonOp::Ne => (ld - rd).abs() >= f64::EPSILON,
                ComparisonOp::Lt => ld < rd,
                ComparisonOp::Le => ld <= rd,
                ComparisonOp::Gt => ld > rd,
                ComparisonOp::Ge => ld >= rd,
                _ => return Err(crate::error::RiskError::TypeMismatch {
                    expected: "decimal-comparable".to_string(),
                    got: "incompatible_op".to_string(),
                }),
            }
        }
        (Value::Boolean { value: l }, Value::Boolean { value: r }) => match op {
            ComparisonOp::Eq => l == r,
            ComparisonOp::Ne => l != r,
            _ => return Err(crate::error::RiskError::TypeMismatch {
                expected: "boolean-comparable".to_string(),
                got: "incompatible_op".to_string(),
            }),
        },
        _ => {
            return Err(crate::error::RiskError::TypeMismatch {
                expected: type_name(right).to_string(),
                got: type_name(left).to_string(),
            })
        }
    };
    Ok(Value::Boolean { value: result })
}

fn evaluate_range(
    value: &Value,
    min: &Value,
    max: &Value,
    inclusive_min: bool,
    inclusive_max: bool,
) -> Result<Value, crate::error::RiskError> {
    let v = to_decimal(value)?;
    let lo = to_decimal(min)?;
    let hi = to_decimal(max)?;
    let above = if inclusive_min { v >= lo } else { v > lo };
    let below = if inclusive_max { v <= hi } else { v < hi };
    Ok(Value::Boolean { value: above && below })
}

fn evaluate_age(date: &Value, as_of: &Value) -> Result<Value, crate::error::RiskError> {
    let (Value::Date { value: d }, Value::Date { value: a }) = (date, as_of) else {
        return Err(crate::error::RiskError::TypeMismatch {
            expected: "date".to_string(),
            got: type_name(date).to_string(),
        });
    };
    let date = chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d")
        .map_err(|e: chrono::ParseError| crate::error::RiskError::Evaluation { message: e.to_string() })?;
    let as_of = chrono::NaiveDate::parse_from_str(a, "%Y-%m-%d")
        .map_err(|e: chrono::ParseError| crate::error::RiskError::Evaluation { message: e.to_string() })?;
    let age = as_of.signed_duration_since(date).num_days();
    Ok(Value::DurationDays { value: age })
}

fn evaluate_duration(from: &Value, to: &Value) -> Result<Value, crate::error::RiskError> {
    let (Value::Date { value: f }, Value::Date { value: t }) = (from, to) else {
        return Err(crate::error::RiskError::TypeMismatch {
            expected: "date".to_string(),
            got: type_name(from).to_string(),
        });
    };
    let from = chrono::NaiveDate::parse_from_str(f, "%Y-%m-%d")
        .map_err(|e: chrono::ParseError| crate::error::RiskError::Evaluation { message: e.to_string() })?;
    let to = chrono::NaiveDate::parse_from_str(t, "%Y-%m-%d")
        .map_err(|e: chrono::ParseError| crate::error::RiskError::Evaluation { message: e.to_string() })?;
    let dur = to.signed_duration_since(from).num_days();
    Ok(Value::DurationDays { value: dur })
}

fn evaluate_arithmetic(
    op: BinaryOp,
    left: &Value,
    right: &Value,
) -> Result<Value, crate::error::RiskError> {
    match (left, right) {
        (Value::Integer { value: l }, Value::Integer { value: r }) => {
            let result = match op {
                BinaryOp::Add => l + r,
                BinaryOp::Sub => l - r,
                BinaryOp::Mul => l * r,
                BinaryOp::Div => {
                    if *r == 0 {
                        return Err(crate::error::RiskError::Evaluation {
                            message: "division by zero".to_string(),
                        });
                    }
                    l / r
                }
            };
            Ok(Value::Integer { value: result })
        }
        (Value::Decimal { value: l }, Value::Decimal { value: r }) => {
            let ld: f64 = l.parse().unwrap_or(0.0);
            let rd: f64 = r.parse().unwrap_or(0.0);
            let result = match op {
                BinaryOp::Add => ld + rd,
                BinaryOp::Sub => ld - rd,
                BinaryOp::Mul => ld * rd,
                BinaryOp::Div => {
                    if rd == 0.0 {
                        return Err(crate::error::RiskError::Evaluation {
                            message: "division by zero".to_string(),
                        });
                    }
                    ld / rd
                }
            };
            Ok(Value::Decimal {
                value: format!("{:.6}", result),
            })
        }
        _ => Err(crate::error::RiskError::TypeMismatch {
            expected: "numeric".to_string(),
            got: type_name(left).to_string(),
        }),
    }
}

fn to_decimal(v: &Value) -> Result<f64, crate::error::RiskError> {
    match v {
        Value::Decimal { value } => value
            .parse::<f64>()
            .map_err(|e: std::num::ParseFloatError| crate::error::RiskError::Evaluation { message: e.to_string() }),
        Value::Integer { value } => Ok(*value as f64),
        Value::DurationDays { value } => Ok(*value as f64),
        _ => Err(crate::error::RiskError::TypeMismatch {
            expected: "numeric".to_string(),
            got: type_name(v).to_string(),
        }),
    }
}
