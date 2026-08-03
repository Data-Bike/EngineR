/**
 * Connector API endpoints (docs/06 §7, docs/14 §4).
 */

import type { ApiClient, ParsedResponse } from '../client/ApiClient.js';
import type {
  ConnectorInstance,
  CreateConnectorBody,
  UpdateConnectorBody,
  ConnectorHealthCheck,
} from '../types/index.js';

/** Connector API endpoint methods. */
export class ConnectorEndpoints {
  constructor(private readonly client: ApiClient) {}

  /** GET /connectors — list connector instances. */
  async list(): Promise<ParsedResponse<ConnectorInstance[]>> {
    return this.client.get<ConnectorInstance[]>('/connectors');
  }

  /** GET /connectors/{id} — get a connector instance. */
  async get(id: string): Promise<ParsedResponse<ConnectorInstance>> {
    return this.client.get<ConnectorInstance>(`/connectors/${id}`);
  }

  /** POST /connectors — create a connector instance. */
  async create(body: CreateConnectorBody): Promise<ParsedResponse<ConnectorInstance>> {
    return this.client.post<ConnectorInstance>('/connectors', body);
  }

  /** PATCH /connectors/{id} — update a connector instance. */
  async update(id: string, body: UpdateConnectorBody): Promise<ParsedResponse<ConnectorInstance>> {
    return this.client.patch<ConnectorInstance>(`/connectors/${id}`, body);
  }

  /** POST /connectors/{id}/enable — enable a connector. */
  async enable(id: string): Promise<ParsedResponse<unknown>> {
    return this.client.post<unknown>(`/connectors/${id}/enable`);
  }

  /** POST /connectors/{id}/disable — disable a connector. */
  async disable(id: string): Promise<ParsedResponse<unknown>> {
    return this.client.post<unknown>(`/connectors/${id}/disable`);
  }

  /** POST /connectors/{id}/health — health check. */
  async healthCheck(id: string): Promise<ParsedResponse<ConnectorHealthCheck>> {
    return this.client.post<ConnectorHealthCheck>(`/connectors/${id}/health`);
  }

  /** DELETE /connectors/{id} — delete a connector instance. */
  async delete(id: string): Promise<ParsedResponse<void>> {
    return this.client.delete<void>(`/connectors/${id}`);
  }
}
