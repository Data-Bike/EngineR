/**
 * Monitoring API endpoints.
 */

import type { ApiClient, ParsedResponse } from '../client/ApiClient.js';
import type { MonitorSchedule, MonitorResult } from '../types/index.js';

/** Monitoring API endpoint methods. */
export class MonitorEndpoints {
  constructor(private readonly client: ApiClient) {}

  /** GET /monitors — list monitor schedules. */
  async list(): Promise<ParsedResponse<MonitorSchedule[]>> {
    return this.client.get<MonitorSchedule[]>('/monitors');
  }

  /** POST /monitors/{id}/run — run a monitor immediately. */
  async run(id: string): Promise<ParsedResponse<MonitorResult>> {
    return this.client.post<MonitorResult>(`/monitors/${id}/run`);
  }
}
