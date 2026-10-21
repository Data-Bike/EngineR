/**
 * Request API endpoints.
 */

import type { ApiClient, ParsedResponse } from '../client/ApiClient.js';

/** Request API endpoint methods. */
export class RequestEndpoints {
  constructor(private readonly client: ApiClient) {}

  /** GET /requests — list requests. */
  async list(query?: { status?: string; limit?: number; cursor?: string }): Promise<ParsedResponse<unknown[]>> {
    return this.client.get<unknown[]>('/requests', query ? { query } : undefined);
  }

  /** POST /requests — create a request. */
  async create(body: { target_entity_id: string; requested_product: string; urgency?: string; due_date?: string }): Promise<ParsedResponse<unknown>> {
    return this.client.post<unknown>('/requests', body);
  }
}
