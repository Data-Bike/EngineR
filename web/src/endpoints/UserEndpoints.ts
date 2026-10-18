/**
 * User administration API endpoints (public admin surface, `users.manage`).
 */

import type { ApiClient, ParsedResponse } from '../client/ApiClient.js';

export interface UserListParams {
  q?: string;
  group_id?: string;
  inherited?: boolean;
  ip?: string;
  status?: string;
  limit?: number;
}

export interface UserInputBody {
  login?: string;
  display_name?: string;
  /** `null` clears the stored email. */
  email?: string | null;
  password?: string;
  status?: string;
  groups?: string[];
  is_service_account?: boolean;
}

/** User admin API endpoint methods. */
export class UserEndpoints {
  constructor(private readonly client: ApiClient) {}

  /** GET /users/groups — role tree flat list (id/parent_id/class). */
  async listRoleGroups(): Promise<ParsedResponse<unknown[]>> {
    return this.client.get<unknown[]>('/users/groups');
  }

  /** GET /users — filtered user list. */
  async list(params: UserListParams = {}): Promise<ParsedResponse<unknown[]>> {
    const qs = new URLSearchParams();
    if (params.q) qs.set('q', params.q);
    if (params.group_id) qs.set('group_id', params.group_id);
    if (params.inherited) qs.set('inherited', '1');
    if (params.ip) qs.set('ip', params.ip);
    if (params.status) qs.set('status', params.status);
    if (params.limit) qs.set('limit', String(params.limit));
    const suffix = qs.size ? `?${qs.toString()}` : '';
    return this.client.get<unknown[]>(`/users${suffix}`);
  }

  /** POST /users — create a user (login + password + roles). */
  async create(body: UserInputBody): Promise<ParsedResponse<unknown>> {
    return this.client.post<unknown>('/users', body);
  }

  /** GET /users/{id} — user detail with roles and sessions. */
  async get(principalId: string): Promise<ParsedResponse<unknown>> {
    return this.client.get<unknown>(`/users/${principalId}`);
  }

  /** PUT /users/{id} — update name/status/roles/password. */
  async update(
    principalId: string,
    body: UserInputBody,
  ): Promise<ParsedResponse<unknown>> {
    return this.client.put<unknown>(`/users/${principalId}`, body);
  }

  /** DELETE /users/{id} — disable the account and end its sessions. */
  async remove(principalId: string): Promise<ParsedResponse<unknown>> {
    return this.client.delete<unknown>(`/users/${principalId}`);
  }

  /** GET /users/{id}/activity — created/changed/viewed objects + audit log. */
  async activity(
    principalId: string,
    q?: string,
    limit?: number,
  ): Promise<ParsedResponse<unknown>> {
    const qs = new URLSearchParams();
    if (q) qs.set('q', q);
    if (limit) qs.set('limit', String(limit));
    const suffix = qs.size ? `?${qs.toString()}` : '';
    return this.client.get<unknown>(`/users/${principalId}/activity${suffix}`);
  }

  /** GET /users/{id}/tokens — API token metadata (never the secret). */
  async listTokens(
    principalId: string,
  ): Promise<ParsedResponse<ApiTokenMeta[]>> {
    return this.client.get<ApiTokenMeta[]>(`/users/${principalId}/tokens`);
  }

  /**
   * POST /users/{id}/tokens — issue an API token. The plaintext `token` is
   * returned exactly once in this response.
   */
  async createToken(
    principalId: string,
    body: { name: string; expires_in_days?: number },
  ): Promise<ParsedResponse<ApiTokenCreated>> {
    return this.client.post<ApiTokenCreated>(
      `/users/${principalId}/tokens`,
      body,
    );
  }

  /** DELETE /users/{id}/tokens/{tokenId} — revoke an API token. */
  async revokeToken(
    principalId: string,
    tokenId: string,
  ): Promise<ParsedResponse<unknown>> {
    return this.client.delete<unknown>(
      `/users/${principalId}/tokens/${tokenId}`,
    );
  }
}

/** API token metadata as returned by the list endpoint. */
export interface ApiTokenMeta {
  token_id: string;
  name: string;
  created_at: string;
  expires_at: string | null;
  revoked_at: string | null;
  last_used_at: string | null;
}

/** Response of token creation — `token` is shown exactly once. */
export interface ApiTokenCreated extends ApiTokenMeta {
  token: string;
}
