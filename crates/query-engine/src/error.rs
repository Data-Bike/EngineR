//! Query engine errors.

use crate::ast::QueryError;
use std::fmt;

/// Alias result.
pub type QueryResult<T> = std::result::Result<T, QueryEngineError>;

/// Query engine error wrapping the AST-level error.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct QueryEngineError(pub QueryError);

impl fmt::Display for QueryEngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for QueryEngineError {}

impl From<QueryError> for QueryEngineError {
    fn from(e: QueryError) -> Self {
        Self(e)
    }
}
