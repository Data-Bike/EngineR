/**
 * Audit API endpoints (docs/08 §11).
 */

import type { ApiClient, ParsedResponse } from '../client/ApiClient.js';
import type {
  AuditRecord,
  AuditQuery,
  ChainCheckpoint,
  RetentionPolicy,
  ChainVerificationResult,
} from '../types/index.js';

/** Audit API endpoint methods. */
export class AuditEndpoints {
  constructor(private readonly client: ApiClient) {}

  /** GET /audit — query audit entries. */
  async query(query: Partial<AuditQuery>): Promise<ParsedResponse<AuditRecord[]>> {
    const params = new URLSearchParams();
    if (query.kind) params.set('kind', query.kind);
    if (query.actor) params.set('actor', query.actor);
    if (query.action) params.set('action', query.action);
    if (query.resource_kind) params.set('resource_kind', query.resource_kind);
    if (query.resource_id) params.set('resource_id', query.resource_id);
    if (query.effect) params.set('effect', query.effect);
    if (query.from) params.set('from', query.from);
    if (query.to) params.set('to', query.to);
    if (query.limit) params.set('limit', String(query.limit));
    if (query.cursor) params.set('cursor', String(query.cursor));
    if (query.catalog) params.set('catalog', query.catalog);
    const qs = params.toString();
    const path = qs ? `/audit?${qs}` : '/audit';
    return this.client.get<AuditRecord[]>(path);
  }

  /** GET /audit/checkpoint — get latest checkpoint. */
  async latestCheckpoint(): Promise<ParsedResponse<ChainCheckpoint | null>> {
    return this.client.get<ChainCheckpoint | null>('/audit/checkpoint');
  }

  /** POST /audit/checkpoint — create a checkpoint. */
  async createCheckpoint(): Promise<ParsedResponse<ChainCheckpoint>> {
    return this.client.post<ChainCheckpoint>('/audit/checkpoint');
  }

  /** POST /audit/verify — verify chain integrity. */
  async verifyChain(): Promise<ParsedResponse<ChainVerificationResult>> {
    return this.client.post<ChainVerificationResult>('/audit/verify');
  }

  /** GET /audit/retention — get retention policy. */
  async getRetentionPolicy(): Promise<ParsedResponse<RetentionPolicy>> {
    return this.client.get<RetentionPolicy>('/audit/retention');
  }

  /** PUT /audit/retention — set retention policy. */
  async setRetentionPolicy(
    policy: Partial<RetentionPolicy>,
  ): Promise<ParsedResponse<RetentionPolicy>> {
    return this.client.put<RetentionPolicy>('/audit/retention', policy);
  }
}
