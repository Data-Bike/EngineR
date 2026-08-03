//! Tracing/metrics initialization (docs/21 §1).

use tracing_subscriber::EnvFilter;

/// Initialize structured JSON tracing with the given default directive.
///
/// Idempotent: safe to call multiple times in tests.
pub fn init(default_directive: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_directive));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .try_init();
}
