/**
 * Schema API endpoints (docs/06 §5).
 */

import type { ApiClient, ParsedResponse } from '../client/ApiClient.js';
import type {
  CreateSchemaDraftBody,
  SchemaDraftTombstone,
  SchemaImpactReport,
  SchemaMigrationStatus,
  SchemaRevisionRecord,
  SchemaValidationReport,
  UpdateSchemaDraftBody,
} from '../types/index.js';

/** Schema API endpoint methods. */
export class SchemaEndpoints {
  constructor(private readonly client: ApiClient) {}

  /** GET /schemas/types — list published types. `catalog` picks one
   * catalog's stream; without it every accessible catalog contributes
   * (each record carries a `catalog` field, docs/28 §3). */
  async listTypes(options?: { catalog?: string }): Promise<ParsedResponse<unknown[]>> {
    return this.client.get<unknown[]>('/schemas/types', { query: options });
  }

  /** GET /schemas/types/{typeId} — get a type definition. `catalog`
   * disambiguates aliases shared between catalogs. */
  async getType(typeId: string, options?: { catalog?: string }): Promise<ParsedResponse<unknown>> {
    return this.client.get<unknown>(`/schemas/types/${typeId}`, { query: options });
  }

  /** GET /schemas/types/{typeId}/view — get a view definition. */
  async getTypeView(typeId: string, viewType: string, options?: { catalog?: string }): Promise<ParsedResponse<unknown>> {
    return this.client.get<unknown>(`/schemas/types/${typeId}/view`, { query: { view_type: viewType, catalog: options?.catalog } });
  }

  /** GET /schemas/drafts — list schema drafts (per-catalog stream when
   * `catalog` is given; always permission-filtered per catalog). */
  async listDrafts(options?: { catalog?: string }): Promise<ParsedResponse<SchemaRevisionRecord[]>> {
    return this.client.get<SchemaRevisionRecord[]>('/schemas/drafts', { query: options });
  }

  /** GET /schemas/drafts/{draftId} — get a schema draft. */
  async getDraft(draftId: string): Promise<ParsedResponse<SchemaRevisionRecord>> {
    return this.client.get<SchemaRevisionRecord>(`/schemas/drafts/${draftId}`);
  }

  /** DELETE /schemas/drafts/{draftId} — delete a schema draft. */
  async deleteDraft(draftId: string, ifMatch: string): Promise<ParsedResponse<SchemaDraftTombstone>> {
    return this.client.delete<SchemaDraftTombstone>(`/schemas/drafts/${draftId}`, { ifMatch });
  }

  /** GET /schemas/published — list published schemas (one stream per
   * catalog; `catalog` filters to a single stream). */
  async listPublished(options?: { catalog?: string }): Promise<ParsedResponse<SchemaRevisionRecord[]>> {
    return this.client.get<SchemaRevisionRecord[]>('/schemas/published', { query: options });
  }

  /** GET /schemas/revisions/{id} — get an immutable revision. */
  async getRevision(revisionId: string): Promise<ParsedResponse<SchemaRevisionRecord>> {
    return this.client.get<SchemaRevisionRecord>(`/schemas/revisions/${revisionId}`);
  }

  /** GET /schemas/predicates — list predicates. `catalog` picks one
   * catalog's stream. */
  async listPredicates(options?: { catalog?: string }): Promise<ParsedResponse<unknown[]>> {
    return this.client.get<unknown[]>('/schemas/predicates', { query: options });
  }

  /** POST /schemas/drafts — create a schema draft inside a catalog's
   * stream (`catalog`, default `general`). */
  async createDraft(body: CreateSchemaDraftBody, options?: { catalog?: string }): Promise<ParsedResponse<SchemaRevisionRecord>> {
    return this.client.post<SchemaRevisionRecord>('/schemas/drafts', body, { query: options });
  }

  /** PUT /schemas/drafts/{id} — update a draft. */
  async updateDraft(draftId: string, body: UpdateSchemaDraftBody, ifMatch: string): Promise<ParsedResponse<SchemaRevisionRecord>> {
    return this.client.put<SchemaRevisionRecord>(`/schemas/drafts/${draftId}`, body, { ifMatch });
  }

  /** POST /schemas/drafts/{id}/validate — validate a draft. */
  async validateDraft(draftId: string): Promise<ParsedResponse<SchemaValidationReport>> {
    return this.client.post<SchemaValidationReport>(`/schemas/drafts/${draftId}/validate`);
  }

  /** POST /schemas/drafts/{id}/impact — impact report. */
  async impactDraft(draftId: string): Promise<ParsedResponse<SchemaImpactReport>> {
    return this.client.post<SchemaImpactReport>(`/schemas/drafts/${draftId}/impact`);
  }

  /** POST /schemas/drafts/{id}/publish — publish a draft. */
  async publishDraft(draftId: string, ifMatch: string, reason: string): Promise<ParsedResponse<SchemaRevisionRecord>> {
    return this.client.post<SchemaRevisionRecord>(
      `/schemas/drafts/${draftId}/publish`,
      { reason },
      { ifMatch },
    );
  }

  /** POST /schemas/revisions/{id}/retire — retire a revision. */
  async retireRevision(revisionId: string, reason: string, ifMatch: string): Promise<ParsedResponse<SchemaRevisionRecord>> {
    return this.client.post<SchemaRevisionRecord>(`/schemas/revisions/${revisionId}/retire`, { reason }, { ifMatch });
  }

  /** GET /schemas/revisions/{id}/migration-status. */
  async migrationStatus(revisionId: string): Promise<ParsedResponse<SchemaMigrationStatus>> {
    return this.client.get<SchemaMigrationStatus>(`/schemas/revisions/${revisionId}/migration-status`);
  }
}
