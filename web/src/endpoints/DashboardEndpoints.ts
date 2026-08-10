/**
 * Dashboard API endpoints.
 */

import type { ApiClient, ParsedResponse } from '../client/ApiClient.js';

/** Dashboard API endpoint methods. */
export class DashboardEndpoints {
  constructor(private readonly client: ApiClient) {}

  /** GET /dashboard/stats — get dashboard summary statistics. */
  async stats(): Promise<ParsedResponse<Record<string, number>>> {
    return this.client.get<Record<string, number>>('/dashboard/stats');
  }
}
