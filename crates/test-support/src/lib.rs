//! Cross-crate invariant tests for EngineR.
//!
//! This crate exists only to host tests that span multiple workspace crates and
//! encode architectural invariants (docs/05 §3). It pulls in the crates needed
//! to exercise the security pipeline, schema registry and identity normalizers
//! end-to-end.

#![forbid(unsafe_code)]

mod invariants;
