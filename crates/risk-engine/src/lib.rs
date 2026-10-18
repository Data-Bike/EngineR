//! Risk engine: criteria DSL, factors, scores, assessment, decision
//! (docs/14 §8-12).
//!
//! The risk engine evaluates typed criterion expressions against entity
//! statements and connector observations. No arbitrary code: the AST is
//! data, evaluated by the engine (docs/14 §8).
//!
//! Pipeline: criterion → evaluate → factor → score → assessment → decision.

#![forbid(unsafe_code)]

pub mod assessment;
pub mod ast;
pub mod criterion;
pub mod decision;
pub mod error;
pub mod factor;
pub mod score;

pub use assessment::{Assessment, AssessmentStatus, ComponentScore, Uncertainty};
pub use ast::{
    evaluate, AstExpr, BinaryOp, ComparisonOp, EvaluationContext, Expr, ExprKind, LogicalOp,
    ObservationContext, UnaryOp, Value,
};
pub use criterion::{Criterion, CriterionSeverity, CriterionStatus, EvidenceSelector};
pub use decision::{Decision, DecisionKind, DecisionReason};
pub use error::{RiskError, RiskResult};
pub use factor::{Factor, FactorStatus, Suppression};
pub use score::{RiskLevel, ScoreConfig, ScoreResult, WeightedAggregator};
