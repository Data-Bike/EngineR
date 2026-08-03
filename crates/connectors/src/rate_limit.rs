//! Rate limiter and circuit breaker (docs/14 §4 step 4).
//!
//! Token bucket rate limiter + exponential backoff retry + circuit breaker
//! with three states: closed, open, half-open.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Rate limiter configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimiterConfig {
    /// Maximum requests per second.
    pub max_rps: u32,
    /// Burst size (max tokens in bucket).
    pub burst: u32,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            max_rps: 10,
            burst: 20,
        }
    }
}

/// Token bucket rate limiter.
#[derive(Debug)]
pub struct RateLimiter {
    config: RateLimiterConfig,
    tokens: AtomicU64,
    last_refill: std::sync::Mutex<Instant>,
}

impl RateLimiter {
    /// Create a new rate limiter.
    pub fn new(config: RateLimiterConfig) -> Self {
        let initial_tokens = config.burst as u64;
        Self {
            config,
            tokens: AtomicU64::new(initial_tokens),
            last_refill: std::sync::Mutex::new(Instant::now()),
        }
    }

    /// Try to acquire one token. Returns `Ok(())` if allowed, `Err(retry_after_ms)` if not.
    pub fn try_acquire(&self) -> Result<(), u64> {
        self.refill();
        let current = self.tokens.load(Ordering::Relaxed);
        if current > 0 {
            self.tokens.fetch_sub(1, Ordering::Relaxed);
            Ok(())
        } else {
            // Calculate retry after: time until next token
            let refill_interval_ms = 1000 / self.config.max_rps as u64;
            Err(refill_interval_ms)
        }
    }

    /// Refill tokens based on elapsed time.
    fn refill(&self) {
        let mut last = self.last_refill.lock().unwrap();
        let now = Instant::now();
        let elapsed = now.duration_since(*last);
        let tokens_to_add =
            (elapsed.as_millis() as u64 * self.config.max_rps as u64) / 1000;
        if tokens_to_add > 0 {
            let current = self.tokens.load(Ordering::Relaxed);
            let new_tokens = (current + tokens_to_add).min(self.config.burst as u64);
            self.tokens.store(new_tokens, Ordering::Relaxed);
            *last = now;
        }
    }

    /// Get current token count (for testing).
    pub fn available_tokens(&self) -> u64 {
        self.refill();
        self.tokens.load(Ordering::Relaxed)
    }
}

/// Circuit breaker state (docs/14 §4 step 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitBreakerState {
    /// Circuit is closed: requests flow normally.
    Closed,
    /// Circuit is open: requests are rejected immediately.
    Open,
    /// Circuit is half-open: a single probe request is allowed.
    HalfOpen,
}

/// Circuit breaker with failure threshold and recovery timeout.
#[derive(Debug)]
pub struct CircuitBreaker {
    state: std::sync::Mutex<CircuitBreakerState>,
    failure_count: AtomicU64,
    failure_threshold: u64,
    recovery_timeout: Duration,
    last_failure_time: std::sync::Mutex<Option<Instant>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker.
    pub fn new(failure_threshold: u64, recovery_timeout_ms: u64) -> Self {
        Self {
            state: std::sync::Mutex::new(CircuitBreakerState::Closed),
            failure_count: AtomicU64::new(0),
            failure_threshold,
            recovery_timeout: Duration::from_millis(recovery_timeout_ms),
            last_failure_time: std::sync::Mutex::new(None),
        }
    }

    /// Check if a request is allowed. Returns `Ok(())` if allowed, `Err` if rejected.
    pub fn allow_request(&self) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        match *state {
            CircuitBreakerState::Closed => Ok(()),
            CircuitBreakerState::Open => {
                // Check if recovery timeout has elapsed
                let last_failure = self.last_failure_time.lock().unwrap();
                if let Some(last) = *last_failure {
                    if last.elapsed() >= self.recovery_timeout {
                        *state = CircuitBreakerState::HalfOpen;
                        return Ok(());
                    }
                }
                Err("circuit breaker is open".to_string())
            }
            CircuitBreakerState::HalfOpen => {
                // Allow one probe request
                Ok(())
            }
        }
    }

    /// Record a successful request.
    pub fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        let mut state = self.state.lock().unwrap();
        *state = CircuitBreakerState::Closed;
    }

    /// Record a failed request.
    pub fn record_failure(&self) {
        let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        let mut last_failure = self.last_failure_time.lock().unwrap();
        *last_failure = Some(Instant::now());
        let mut state = self.state.lock().unwrap();
        if count >= self.failure_threshold {
            *state = CircuitBreakerState::Open;
        } else if *state == CircuitBreakerState::HalfOpen {
            *state = CircuitBreakerState::Open;
        }
    }

    /// Get current state (for testing).
    pub fn state(&self) -> CircuitBreakerState {
        *self.state.lock().unwrap()
    }

    /// Get current failure count (for testing).
    pub fn failure_count(&self) -> u64 {
        self.failure_count.load(Ordering::Relaxed)
    }
}

/// Calculate exponential backoff delay for a retry attempt.
pub fn backoff_delay(attempt: u32, base_ms: u64, max_ms: u64) -> Duration {
    let delay = base_ms * (1u64 << attempt.min(10));
    Duration::from_millis(delay.min(max_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limiter_allows_within_burst() {
        let limiter = RateLimiter::new(RateLimiterConfig {
            max_rps: 100,
            burst: 10,
        });
        for _ in 0..10 {
            assert!(limiter.try_acquire().is_ok());
        }
        assert!(limiter.try_acquire().is_err());
    }

    #[test]
    fn circuit_breaker_opens_after_threshold() {
        let cb = CircuitBreaker::new(3, 1000);
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
        assert!(cb.allow_request().is_err());
    }

    #[test]
    fn circuit_breaker_resets_on_success() {
        let cb = CircuitBreaker::new(3, 1000);
        cb.record_failure();
        cb.record_failure();
        cb.record_success();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn backoff_delay_exponential() {
        assert_eq!(backoff_delay(0, 100, 10000), Duration::from_millis(100));
        assert_eq!(backoff_delay(1, 100, 10000), Duration::from_millis(200));
        assert_eq!(backoff_delay(2, 100, 10000), Duration::from_millis(400));
        assert_eq!(backoff_delay(3, 100, 10000), Duration::from_millis(800));
    }

    #[test]
    fn backoff_delay_capped() {
        assert_eq!(backoff_delay(10, 100, 1000), Duration::from_millis(1000));
    }
}
