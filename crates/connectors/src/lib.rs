//! Connector SDK: source adapters, rate limit, raw payload/provenance
//! (docs/14).
//!
//! A connector is a source adapter implementing:
//! - descriptor/capabilities;
//! - credential validation;
//! - health/quota;
//! - search/fetch/monitor operations;
//! - request/response schema versions;
//! - normalization mapping;
//! - rate limit/retry;
//! - legal/licensing metadata;
//! - redacted diagnostics.
//!
//! The core never knows the fields of a specific API. A connector returns
//! [`SourceObservation`] and a mapping result (docs/14 §1).

#![forbid(unsafe_code)]

pub mod descriptor;
pub mod error;
pub mod instance;
pub mod observation;
pub mod rate_limit;
pub mod sdk;

pub use descriptor::{ConnectorCapabilities, ConnectorDescriptor, ConnectorKind, LegalMetadata};
pub use error::{ConnectorError, ConnectorResult};
pub use instance::{ConnectorInstance, ConnectorInstanceConfig, RetentionPolicy, TrustRank};
pub use observation::{
    IdentityResolution, MappedPredicate, MappingResult, ObservationStatus, ProvenanceMeta,
    RawPayloadPolicy, SourceObservation, UnmappedField,
};
pub use rate_limit::{CircuitBreaker, CircuitBreakerState, RateLimiter, RateLimiterConfig};
pub use sdk::{Connector, ConnectorContext, SearchQuery, SearchResponse};
