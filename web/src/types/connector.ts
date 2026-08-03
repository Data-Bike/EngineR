/**
 * Connector API types (docs/06 §7, docs/14 §4).
 */

/** Connector instance response. */
export interface ConnectorInstance {
  instance_id: string;
  connector_type_id: string;
  tenant_id: string;
  enabled: boolean;
  environment: string;
  rate_limit_rps: number;
  trust_rank: number;
}

/** Create connector request body. */
export interface CreateConnectorBody {
  connector_type_id: string;
  environment: string;
  base_url?: string;
  rate_limit_rps?: number;
  concurrency?: number;
  timeout_seconds?: number;
  trust_rank?: number;
}

/** Update connector request body. */
export interface UpdateConnectorBody {
  enabled?: boolean;
  rate_limit_rps?: number;
  concurrency?: number;
  timeout_seconds?: number;
}

/** Health check response. */
export interface ConnectorHealthCheck {
  healthy: boolean;
  latency_ms: number;
  message: string;
}

/** Trust rank levels (docs/14 §1). */
export const TrustRank = {
  Untrusted: 0,
  Low: 25,
  Medium: 50,
  High: 75,
  Authoritative: 100,
} as const;

export type TrustRank = (typeof TrustRank)[keyof typeof TrustRank];
