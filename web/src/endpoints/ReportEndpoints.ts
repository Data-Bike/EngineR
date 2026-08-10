/**
 * Reporting API endpoints (docs/15 §9).
 */

import type { ApiClient, ParsedResponse } from '../client/ApiClient.js';
import type { ReportInstance } from '../types/index.js';

/** Reports API endpoint methods. */
export class ReportEndpoints {
  constructor(private readonly client: ApiClient) {}

  /** GET /reports — list reports. */
  async list(): Promise<ParsedResponse<ReportInstance[]>> {
    return this.client.get<ReportInstance[]>('/reports');
  }

  /** POST /reports — create a report. */
  async create(body: { entity_id: string; template: string; format: string }): Promise<ParsedResponse<ReportInstance>> {
    return this.client.post<ReportInstance>('/reports', body);
  }
}
