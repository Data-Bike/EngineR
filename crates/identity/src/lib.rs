//! Identity: normalizers, identity keys, match candidates (docs/04 §6-7).
//!
//! Normalization is idempotent: `normalize(normalize(x)) == normalize(x)`
//! (docs/04 §13). Raw, canonical and normalizer-version are all stored.

#![forbid(unsafe_code)]

pub mod normalizers;

pub use normalizers::{
    normalize_email, normalize_phone, normalize_text, normalize_text_search_key,
    NormalizationError, NormalizedEmail, NormalizedPhone, NormalizedText,
};
