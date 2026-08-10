/**
 * Security/permissions admin API endpoints (public admin surface).
 */

import type { ApiClient, ParsedResponse } from '../client/ApiClient.js';

/** Security admin API endpoint methods. */
export class SecurityEndpoints {
  constructor(private readonly client: ApiClient) {}

  /** GET /permissions/groups — list permission groups. */
  async listGroups(): Promise<ParsedResponse<unknown[]>> {
    return this.client.get<unknown[]>('/permissions/groups');
  }

  /** POST /permissions/groups — create a permission group. */
  async createGroup(body: unknown): Promise<ParsedResponse<unknown>> {
    return this.client.post<unknown>('/permissions/groups', body);
  }

  /** PUT /permissions/groups/{groupId} — update a permission group. */
  async updateGroup(groupId: string, body: unknown): Promise<ParsedResponse<unknown>> {
    return this.client.put<unknown>(`/permissions/groups/${groupId}`, body);
  }

  /** DELETE /permissions/groups/{groupId} — delete a permission group. */
  async deleteGroup(groupId: string): Promise<ParsedResponse<unknown>> {
    return this.client.delete<unknown>(`/permissions/groups/${groupId}`);
  }
}
