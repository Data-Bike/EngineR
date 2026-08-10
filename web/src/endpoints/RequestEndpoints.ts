/**
 * Request API endpoints.
 */

import type { ApiClient, ParsedResponse } from '../client/ApiClient.js';

/** Request API endpoint methods. */
export class RequestEndpoints {
  constructor(private readonly client: ApiClient) {}

  /** GET /requests — list requests. */
  async list(): Promise<ParsedResponse<unknown[]>> {
    return this.client.get<unknown[]>('/requests');
  }

  /** POST /requests — create a request. */
  async create(body: { target_entity_id: string; requested_product: string; urgency?: string; due_date?: string }): Promise<ParsedResponse<unknown>> {
    return this.client.post<unknown>('/requests', body);
  }
}
