//! Stable HTTP DTO, error codes and API envelope (docs/06).
#![forbid(unsafe_code)]

pub mod envelope;
pub mod error;
pub mod import;

pub use envelope::{ApiEnvelope, ApiLinks, ApiMeta};
pub use error::{ApiError, ApiErrorCode};
pub use import::{
    CellParser, ColumnMapping, ConflictAction, DryRunRowResult, DryRunSummary, EmptySemantics,
    ImportError, MappingProfile, ParsedCell, ParsedRow, PlannedStatement, RelationMapping,
    RowOutcome,
};
