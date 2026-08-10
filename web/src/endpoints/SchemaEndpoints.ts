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

  /** GET /schemas/types/{typeId}/view — get a view definition. */
  async getTypeView(typeId: string, viewType: string): Promise<ParsedResponse<unknown>> {
    return this.client.get<unknown>(`/schemas/types/${typeId}/view`, { query: { view_type: viewType } });
  }

  /** POST /schemas/types — create a type. */
  async createType(body: unknown): Promise<ParsedResponse<unknown>> {
    return this.client.post<unknown>('/schemas/types', body);
  }

  /** PUT /schemas/types/{typeId} — update a type. */
  async updateType(typeId: string, body: unknown): Promise<ParsedResponse<unknown>> {
    return this.client.put<unknown>(`/schemas/types/${typeId}`, body);
  }

  /** DELETE /schemas/types/{typeId} — delete a type. */
  async deleteType(typeId: string): Promise<ParsedResponse<unknown>> {
    return this.client.delete<unknown>(`/schemas/types/${typeId}`);
  }

  /** GET /schemas/drafts — list schema drafts. */
  async listDrafts(): Promise<ParsedResponse<unknown[]>> {
    return this.client.get<unknown[]>('/schemas/drafts');
  }

  /** GET /schemas/drafts/{draftId} — get a schema draft. */
  async getDraft(draftId: string): Promise<ParsedResponse<unknown>> {
    return this.client.get<unknown>(`/schemas/drafts/${draftId}`);
  }

  /** DELETE /schemas/drafts/{draftId} — delete a schema draft. */
  async deleteDraft(draftId: string): Promise<ParsedResponse<unknown>> {
    return this.client.delete<unknown>(`/schemas/drafts/${draftId}`);
  }

  /** GET /schemas/published — list published schemas. */
  async listPublished(): Promise<ParsedResponse<unknown[]>> {
    return this.client.get<unknown[]>('/schemas/published');
  }

  /** GET /schemas/predicates — list predicates. */
  async listPredicates(): Promise<ParsedResponse<unknown[]>> {
    return this.client.get<unknown[]>('/schemas/predicates');
  }

  /** POST /schemas/predicates — create a predicate. */
  async createPredicate(body: unknown): Promise<ParsedResponse<unknown>> {
    return this.client.post<unknown>('/schemas/predicates', body);
  }

  /** PUT /schemas/predicates/{alias} — update a predicate. */
  async updatePredicate(alias: string, body: unknown): Promise<ParsedResponse<unknown>> {
    return this.client.put<unknown>(`/schemas/predicates/${alias}`, body);
  }

  /** DELETE /schemas/predicates/{alias} — delete a predicate. */
  async deletePredicate(alias: string): Promise<ParsedResponse<unknown>> {
    return this.client.delete<unknown>(`/schemas/predicates/${alias}`);
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

  /** POST /schemas/drafts/{id}/publish — publish a draft. */
  async publishDraft(draftId: string, ifMatch?: string, reason?: string): Promise<ParsedResponse<SchemaRevision>> {
    return this.client.post<SchemaRevision>(
      `/schemas/drafts/${draftId}/publish`,
      { reason },
      { ifMatch },
    );
  }

  /** POST /schemas/drafts/{id}/retire — retire a draft. */
  async retireDraft(draftId: string, reason?: string): Promise<ParsedResponse<unknown>> {
    return this.client.post<unknown>(`/schemas/drafts/${draftId}/retire`, { reason });
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
