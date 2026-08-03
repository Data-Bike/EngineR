/**
 * Schema API endpoints (docs/06 §5).
 */

import type { ApiClient, ParsedResponse } from '../client/ApiClient.js';
import type { SchemaRevision } from '../types/index.js';

/** Schema API endpoint methods. */
export class SchemaEndpoints {
  constructor(private readonly client: ApiClient) {}

  /** GET /schemas/types — list published types. */
  async listTypes(): Promise<ParsedResponse<unknown[]>> {
    return this.client.get<unknown[]>('/schemas/types');
  }

  /** GET /schemas/types/{typeId} — get a type definition. */
  async getType(typeId: string): Promise<ParsedResponse<unknown>> {
    return this.client.get<unknown>(`/schemas/types/${typeId}`);
  }

  /** POST /schemas/drafts — create a schema draft. */
  async createDraft(body: unknown): Promise<ParsedResponse<unknown>> {
    return this.client.post<unknown>('/schemas/drafts', body);
  }

  /** PUT /schemas/drafts/{id} — update a draft. */
  async updateDraft(draftId: string, body: unknown, ifMatch?: string): Promise<ParsedResponse<unknown>> {
    return this.client.put<unknown>(`/schemas/drafts/${draftId}`, body, { ifMatch });
  }

  /** POST /schemas/drafts/{id}/validate — validate a draft. */
  async validateDraft(draftId: string): Promise<ParsedResponse<unknown>> {
    return this.client.post<unknown>(`/schemas/drafts/${draftId}/validate`);
  }

  /** POST /schemas/drafts/{id}/impact — impact report. */
  async impactDraft(draftId: string): Promise<ParsedResponse<unknown>> {
    return this.client.post<unknown>(`/schemas/drafts/${draftId}/impact`);
  }

  /** POST /schemas/drafts/{id}/publish — publish a draft (requires If-Match). */
  async publishDraft(draftId: string, ifMatch: string, reason: string): Promise<ParsedResponse<SchemaRevision>> {
    return this.client.post<SchemaRevision>(
      `/schemas/drafts/${draftId}/publish`,
      { reason },
      { ifMatch },
    );
  }

  /** POST /schemas/revisions/{id}/retire — retire a revision. */
  async retireRevision(revisionId: string, reason: string): Promise<ParsedResponse<unknown>> {
    return this.client.post<unknown>(`/schemas/revisions/${revisionId}/retire`, { reason });
  }

  /** GET /schemas/revisions/{id}/migration-status. */
  async migrationStatus(revisionId: string): Promise<ParsedResponse<unknown>> {
    return this.client.get<unknown>(`/schemas/revisions/${revisionId}/migration-status`);
  }
}
