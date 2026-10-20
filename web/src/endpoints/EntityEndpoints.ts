/**
 * Entity API endpoints (docs/06 §6).
 */

import type { ApiClient } from '../client/ApiClient.js';
import type { ParsedResponse } from '../client/ApiClient.js';
import type {
  CreateEntityBody,
  CreateEntityResponse,
  Entity,
  EntityGraphResponse,
  EntityRelation,
  MergeEntityBody,
  MergeEntityResponse,
  QueryBody,
  QueryResponse,
  RetireEntityBody,
  RetireEntityResponse,
  UpdateEntityBody,
} from '../types/index.js';

/** Entity API endpoint methods. */
export class EntityEndpoints {
  constructor(private readonly client: ApiClient) {}

  /** POST /entities — create entity (docs/06 §6). `catalog` selects the
   * owning catalog; without it the type's own catalog is used (docs/28). */
  async create(body: CreateEntityBody, options?: { idempotencyKey?: string; catalog?: string }): Promise<ParsedResponse<CreateEntityResponse>> {
    return this.client.post<CreateEntityResponse>('/entities', body, {
      idempotencyKey: options?.idempotencyKey,
      query: { catalog: options?.catalog },
    });
  }

  /** GET /entities — list entities (type filter + prefix search).
   * `catalog` scopes the listing to one catalog (docs/28 §6). */
  async list(options?: {
    type?: string;
    search?: string;
    limit?: number;
    cursor?: string;
    catalog?: string;
  }): Promise<ParsedResponse<Entity[]>> {
    return this.client.get<Entity[]>('/entities', { query: options });
  }

  /** GET /entities/{publicId} — get entity. `catalog` asserts the active
   * catalog: a foreign-catalog entity reads as 404. */
  async get(publicId: string, options?: { catalog?: string }): Promise<ParsedResponse<Entity>> {
    return this.client.get<Entity>(`/entities/${publicId}`, { query: options });
  }

  /** POST /entities/query — query entities (docs/06 §9). `catalog` scopes
   * the query to one catalog. */
  async query(body: QueryBody, options?: { catalog?: string }): Promise<ParsedResponse<QueryResponse>> {
    return this.client.post<QueryResponse>('/entities/query', body, { query: options });
  }

  /** GET /entities/{publicId}/relations — list entity relations. */
  async getRelations(
    publicId: string,
    options?: { direction?: 'outgoing' | 'incoming' | 'both'; as_of?: string },
  ): Promise<ParsedResponse<EntityRelation[]>> {
    return this.client.get<EntityRelation[]>(`/entities/${publicId}/relations`, { query: options });
  }

  /** POST /entities/{publicId}/relations — add a relation. */
  async addRelation(publicId: string, body: unknown): Promise<ParsedResponse<unknown>> {
    return this.client.post<unknown>(`/entities/${publicId}/relations`, body);
  }

  /** DELETE /entities/{publicId}/relations/{relationId} — remove a relation. */
  async deleteRelation(publicId: string, relationId: string): Promise<ParsedResponse<unknown>> {
    return this.client.delete<unknown>(`/entities/${publicId}/relations/${relationId}`);
  }

  /** PUT /entities/{publicId} — update entity statements. `catalog` is a
   * mismatch check: entities never change catalogs (409 catalog.mismatch). */
  async update(publicId: string, body: UpdateEntityBody, options?: { catalog?: string }): Promise<ParsedResponse<Entity>> {
    return this.client.put<Entity>(`/entities/${publicId}`, body, { query: options });
  }

  /** GET /entities/{publicId}/history — get entity history. */
  async getHistory(publicId: string): Promise<ParsedResponse<unknown[]>> {
    return this.client.get<unknown[]>(`/entities/${publicId}/history`);
  }

  /** POST /entities/{publicId}/retire — retire entity. */
  async retire(publicId: string, body: RetireEntityBody): Promise<ParsedResponse<RetireEntityResponse>> {
    return this.client.post<RetireEntityResponse>(`/entities/${publicId}/retire`, body);
  }

  /** POST /entities/{publicId}/merge — merge entities (docs/04 §11). */
  async merge(publicId: string, body: MergeEntityBody): Promise<ParsedResponse<MergeEntityResponse>> {
    return this.client.post<MergeEntityResponse>(`/entities/${publicId}/merge`, body);
  }

  /** POST /entities/{publicId}/graph — expand entity graph (docs/17 §7). */
  async getGraph(
    publicId: string,
    body: { max_depth: number; direction: string; as_of?: string; locale?: string },
  ): Promise<ParsedResponse<EntityGraphResponse>> {
    return this.client.post<EntityGraphResponse>(`/entities/${publicId}/graph`, body);
  }
}
