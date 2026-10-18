//! Connector SDK tests (docs/14).

#![cfg(test)]

use enginer_connectors::{
    CircuitBreaker, CircuitBreakerState, ConnectorCapabilities, ConnectorDescriptor,
    ConnectorInstance, ConnectorInstanceConfig, ConnectorKind, LegalMetadata, RateLimiter,
    RateLimiterConfig, RetentionPolicy, TrustRank,
};
use std::sync::Arc;

fn make_descriptor() -> ConnectorDescriptor {
    ConnectorDescriptor {
        connector_type_id: "fns-registry".to_string(),
        name: "ФНС Registry".to_string(),
        version: "1.0.0".to_string(),
        kind: ConnectorKind::Api,
        capabilities: ConnectorCapabilities {
            search: true,
            fetch: true,
            monitor: true,
            file_import: false,
            supported_target_types: vec!["legal_entity".to_string()],
            supported_check_kinds: vec!["registration".to_string()],
            max_response_bytes: 5 * 1024 * 1024,
            retains_raw_payload: true,
        },
        legal: LegalMetadata {
            legal_basis: "legitimate_interest".to_string(),
            purpose: "kyc".to_string(),
            jurisdiction: "RU".to_string(),
            license_ref: Some("contract-001".to_string()),
            retention_days: Some(365),
            redistribution_allowed: false,
        },
        request_schema_version: "1.0".to_string(),
        response_schema_version: "1.0".to_string(),
        mapping_package_version: "1.0.0".to_string(),
        trust_rank: 75,
    }
}

fn make_config() -> ConnectorInstanceConfig {
    ConnectorInstanceConfig {
        instance_id: "inst-001".to_string(),
        tenant_id: "tenant-001".to_string(),
        connector_type_id: "fns-registry".to_string(),
        connector_version: "1.0.0".to_string(),
        enabled: true,
        enabled_operations: vec!["search".to_string(), "fetch".to_string()],
        environment: "production".to_string(),
        base_url: Some("https://api.fns.example.com".to_string()),
        proxy_url: None,
        secret_references: vec!["vault://fns/api-key".to_string()],
        organization_account: Some("org-001".to_string()),
        rate_limit_rps: 10,
        concurrency_limit: 5,
        timeout_ms: 30000,
        retry_count: 3,
        retry_backoff_ms: 100,
        cache_freshness_seconds: 3600,
        legal_basis: "legitimate_interest".to_string(),
        purpose: "kyc".to_string(),
        permitted_target_types: vec!["legal_entity".to_string()],
        permitted_target_fields: vec!["legal_entity.inn".to_string()],
        retention: RetentionPolicy::default(),
        trust_rank: TrustRank::High,
        mapping_package_version: "1.0.0".to_string(),
    }
}

#[test]
fn descriptor_has_correct_kind() {
    let d = make_descriptor();
    assert_eq!(d.kind, ConnectorKind::Api);
    assert!(d.capabilities.search);
    assert!(d.capabilities.fetch);
    assert!(!d.capabilities.file_import);
}

#[test]
fn descriptor_legal_metadata() {
    let d = make_descriptor();
    assert_eq!(d.legal.jurisdiction, "RU");
    assert!(!d.legal.redistribution_allowed);
    assert_eq!(d.legal.retention_days, Some(365));
}

#[test]
fn instance_quota_tracking() {
    let config = make_config();
    let mut inst = ConnectorInstance::new(config);
    assert!(inst.has_quota());
    assert!(inst.consume_quota());
    assert!(inst.consume_quota());
    assert_eq!(inst.quota_used, 2);
    inst.reset_quota();
    assert_eq!(inst.quota_used, 0);
}

#[test]
fn instance_health_tracking() {
    let config = make_config();
    let mut inst = ConnectorInstance::new(config);
    assert!(inst.healthy);
    inst.set_health(false);
    assert!(!inst.healthy);
    assert!(inst.last_health_check.is_some());
}

#[test]
fn trust_rank_values() {
    assert_eq!(TrustRank::Untrusted.rank(), 0);
    assert_eq!(TrustRank::Low.rank(), 25);
    assert_eq!(TrustRank::Medium.rank(), 50);
    assert_eq!(TrustRank::High.rank(), 75);
    assert_eq!(TrustRank::Authoritative.rank(), 100);
}

#[test]
fn rate_limiter_burst() {
    let limiter = RateLimiter::new(RateLimiterConfig {
        max_rps: 100,
        burst: 5,
    });
    for _ in 0..5 {
        assert!(limiter.try_acquire().is_ok());
    }
    assert!(limiter.try_acquire().is_err());
}

#[test]
fn circuit_breaker_transitions() {
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
fn circuit_breaker_success_resets() {
    let cb = CircuitBreaker::new(3, 1000);
    cb.record_failure();
    cb.record_failure();
    cb.record_success();
    assert_eq!(cb.state(), CircuitBreakerState::Closed);
    assert_eq!(cb.failure_count(), 0);
}

#[test]
fn backoff_delay_exponential() {
    use enginer_connectors::rate_limit::backoff_delay;
    use std::time::Duration;
    assert_eq!(backoff_delay(0, 100, 10000), Duration::from_millis(100));
    assert_eq!(backoff_delay(1, 100, 10000), Duration::from_millis(200));
    assert_eq!(backoff_delay(2, 100, 10000), Duration::from_millis(400));
    assert_eq!(backoff_delay(3, 100, 10000), Duration::from_millis(800));
}

#[test]
fn backoff_delay_capped() {
    use enginer_connectors::rate_limit::backoff_delay;
    use std::time::Duration;
    assert_eq!(backoff_delay(10, 100, 1000), Duration::from_millis(1000));
}

#[test]
fn retention_policy_default() {
    let r = RetentionPolicy::default();
    assert_eq!(r.raw_payload_retention_days, 90);
    assert_eq!(r.observation_retention_days, 365);
    assert!(!r.legal_hold);
    assert!(r.cryptographic_delete);
}

#[test]
fn connector_descriptor_trust_rank() {
    let d = make_descriptor();
    assert_eq!(d.trust_rank, 75);
}

#[test]
fn instance_from_config() {
    let config = make_config();
    let inst = ConnectorInstance::new(config.clone());
    assert_eq!(inst.config.instance_id, "inst-001");
    assert_eq!(inst.config.rate_limit_rps, 10);
    assert!(inst.healthy);
    assert_eq!(inst.quota_used, 0);
}

#[test]
fn capabilities_default() {
    let caps = ConnectorCapabilities::default();
    assert!(caps.search);
    assert!(caps.fetch);
    assert!(!caps.monitor);
    assert_eq!(caps.max_response_bytes, 10 * 1024 * 1024);
}
