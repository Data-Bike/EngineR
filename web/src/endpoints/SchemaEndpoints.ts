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
  async listDrafts(): Promise<ParsedResponse<SchemaRevisionRecord[]>> {
    return this.client.get<SchemaRevisionRecord[]>('/schemas/drafts');
  }

  /** GET /schemas/drafts/{draftId} — get a schema draft. */
  async getDraft(draftId: string): Promise<ParsedResponse<SchemaRevisionRecord>> {
    return this.client.get<SchemaRevisionRecord>(`/schemas/drafts/${draftId}`);
  }

  /** DELETE /schemas/drafts/{draftId} — delete a schema draft. */
  async deleteDraft(draftId: string, ifMatch: string): Promise<ParsedResponse<SchemaDraftTombstone>> {
    return this.client.delete<SchemaDraftTombstone>(`/schemas/drafts/${draftId}`, { ifMatch });
  }

  /** GET /schemas/published — list published schemas. */
  async listPublished(): Promise<ParsedResponse<SchemaRevisionRecord[]>> {
    return this.client.get<SchemaRevisionRecord[]>('/schemas/published');
  }

  /** GET /schemas/revisions/{id} — get an immutable revision. */
  async getRevision(revisionId: string): Promise<ParsedResponse<SchemaRevisionRecord>> {
    return this.client.get<SchemaRevisionRecord>(`/schemas/revisions/${revisionId}`);
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
  async createDraft(body: CreateSchemaDraftBody): Promise<ParsedResponse<SchemaRevisionRecord>> {
    return this.client.post<SchemaRevisionRecord>('/schemas/drafts', body);
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
