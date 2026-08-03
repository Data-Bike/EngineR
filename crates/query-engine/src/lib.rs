//! Query engine: AST, security rewrite and SQL compiler (docs/07).
//!
//! Pipeline (docs/07 §4):
//! 1. parse + syntactic limits;
//! 2. resolve schema revision;
//! 3. semantic type-check;
//! 4. normalize literals;
//! 5. request capability check;
//! 6. PDP decision;
//! 7. security AST rewrite;
//! 8. projection/index selection;
//! 9. cost estimate;
//! 10. parameterized SQL compile;
//! 11. execution deadline/cancellation;
//! 12. response allowlist;
//! 13. audit.
//!
//! This crate implements steps 1-3, 7, 9-10 as pure functions over the AST and
//! the PDP decision. Steps 5-6 (capability/PDP) and 11-13 (execution/audit) are
//! the caller's responsibility. The compiler produces parameterized SQL with
//! bind placeholders — no user literal ever appears in the SQL text
//! (docs/07 §6, docs/07 §14).

#![forbid(unsafe_code)]

pub mod ast;
pub mod compile;
pub mod error;
pub mod rewrite;

pub use ast::{
    FieldRef, Operator, QueryAst, QueryError, SelectField, SortDirection, SortKey, WhereExpr,
};
pub use compile::{CompiledQuery, SqlFragment};
pub use error::{QueryEngineError, QueryResult};
pub use rewrite::SecurityRewriter;
