/**
 * Health check endpoints (docs/02 §6).
 */

import type { ApiClient, ParsedResponse } from '../client/ApiClient.js';

/** Health check response. */
export interface HealthResponse {
  status: 'ok' | 'degraded' | 'down';
  version?: string;
  dependencies?: Record<string, string>;
}

/** Health endpoint methods. */
export class HealthEndpoints {
  constructor(private readonly client: ApiClient) {}

  /** GET /health/live — liveness probe. */
  async live(): Promise<ParsedResponse<HealthResponse>> {
    return this.client.get<HealthResponse>('/health/live');
  }

  /** GET /health/ready — readiness probe. */
  async ready(): Promise<ParsedResponse<HealthResponse>> {
    return this.client.get<HealthResponse>('/health/ready');
  }
}
