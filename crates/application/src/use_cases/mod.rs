//! Application use cases (docs/05 §5, docs/04 §10-11, docs/06 §6).
//!
//! Each use case is a pure orchestrator over ports. It does not know about
//! Rocket, sqlx or JSON — it takes typed commands and returns typed results.
//! The API layer translates wire DTOs into commands and results into envelopes.

pub mod create_entity;
pub mod merge_entity;
pub mod retire_entity;

pub use create_entity::{CreateEntity, CreateEntityCommand, CreateEntityResult, DedupOutcome};
pub use merge_entity::{MergeEntity, MergeEntityCommand, MergeEntityResult};
pub use retire_entity::{RetireEntity, RetireEntityCommand, RetireEntityResult};
