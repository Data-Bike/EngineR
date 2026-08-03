/**
 * Entity API endpoints (docs/06 §6).
 */

import type { ApiClient } from '../client/ApiClient.js';
import type { ParsedResponse } from '../client/ApiClient.js';
import type {
  CreateEntityBody,
  CreateEntityResponse,
  Entity,
  MergeEntityBody,
  MergeEntityResponse,
  QueryBody,
  QueryResponse,
  RetireEntityBody,
  RetireEntityResponse,
} from '../types/index.js';

/** Entity API endpoint methods. */
export class EntityEndpoints {
  constructor(private readonly client: ApiClient) {}

  /** POST /entities — create entity (docs/06 §6). */
  async create(body: CreateEntityBody, idempotencyKey?: string): Promise<ParsedResponse<CreateEntityResponse>> {
    return this.client.post<CreateEntityResponse>('/entities', body, { idempotencyKey });
  }

  /** GET /entities/{publicId} — get entity. */
  async get(publicId: string): Promise<ParsedResponse<Entity>> {
    return this.client.get<Entity>(`/entities/${publicId}`);
  }

  /** POST /entities/query — query entities (docs/06 §9). */
  async query(body: QueryBody): Promise<ParsedResponse<QueryResponse>> {
    return this.client.post<QueryResponse>('/entities/query', body);
  }

  /** POST /entities/{publicId}/retire — retire entity. */
  async retire(publicId: string, body: RetireEntityBody): Promise<ParsedResponse<RetireEntityResponse>> {
    return this.client.post<RetireEntityResponse>(`/entities/${publicId}/retire`, body);
  }

  /** POST /entities/{publicId}/merge — merge entities (docs/04 §11). */
  async merge(publicId: string, body: MergeEntityBody): Promise<ParsedResponse<MergeEntityResponse>> {
    return this.client.post<MergeEntityResponse>(`/entities/${publicId}/merge`, body);
  }
}
