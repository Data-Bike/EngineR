/**
 * Audit API types (docs/08 §11).
 */

/** Audit record. */
export interface AuditRecord {
  audit_id: number;
  sequence: number;
  previous_hash: string;
  entry_hash: string;
  tenant_id: string;
  occurred_at: string;
  actor: string;
  effective_actor: string | null;
  kind: AuditKind;
  action: string;
  resource_kind: string;
  resource_id: string | null;
  effect: string | null;
  reason_ids: string[];
  policy_revision: number | null;
  request_id: string | null;
  correlation_id: string | null;
  query_fingerprint: string | null;
  row_count: number | null;
  bytes: number | null;
  client: string | null;
  service: string | null;
  metadata: Record<string, unknown>;
}

/** Audit kind (docs/08 §11). */
export const AuditKind = {
  Authentication: 'authentication',
  Decision: 'decision',
  DeniedRequest: 'denied_request',
  PolicyChange: 'policy_change',
  Export: 'export',
  BlobRead: 'blob_read',
  BulkAction: 'bulk_action',
  BreakGlass: 'break_glass',
  Impersonation: 'impersonation',
} as const;

export type AuditKind = (typeof AuditKind)[keyof typeof AuditKind];

/** Audit query parameters. */
export interface AuditQuery {
  tenant_id: string;
  kind?: AuditKind;
  actor?: string;
  action?: string;
  resource_kind?: string;
  resource_id?: string;
  effect?: string;
  from?: string;
  to?: string;
  limit?: number;
  cursor?: number;
}

/** Chain checkpoint. */
export interface ChainCheckpoint {
  checkpoint_id: number;
  tenant_id: string;
  sequence: number;
  entry_hash: string;
  created_at: string;
}

/** Retention policy. */
export interface RetentionPolicy {
  tenant_id: string;
  retention_days: number;
  legal_hold: boolean;
  archive_before_delete: boolean;
  archive_url: string | null;
}

/** Chain verification result. */
export interface ChainVerificationResult {
  valid: boolean;
  entry_count: number;
  last_sequence: number;
  last_hash: string;
}
