/**
 * @enginer/api-client — Typed API client for EngineR (docs/06).
 *
 * Usage:
 * ```ts
 * import { createEnginerClient } from '@enginer/api-client';
 *
 * const api = createEnginerClient({
 *   baseUrl: 'http://localhost:8000/api/v1',
 *   locale: 'ru-RU',
 * });
 *
 * const { data, meta } = await api.entities.create({
 *   type_id: '...',
 *   schema_revision: 1,
 *   identity_mode: 'return_existing',
 *   statements: [],
 * });
 * ```
 */

import { ApiClient, type ApiClientConfig, type ParsedResponse, type RequestOptions } from './client/ApiClient.js';
import { EntityEndpoints, SchemaEndpoints, SecurityEndpoints, HealthEndpoints, ConnectorEndpoints, CheckEndpoints, FileEndpoints, AuditEndpoints, ReportEndpoints, DashboardEndpoints, RequestEndpoints, MonitorEndpoints, UserEndpoints } from './endpoints/index.js';

export * from './types/index.js';
export { ApiClient, type ApiClientConfig, type ParsedResponse, type RequestOptions };
export { ApiClientError, type ApiError, type ApiErrorCode, httpStatusForCode } from './types/error.js';
export { EntityEndpoints, SchemaEndpoints, SecurityEndpoints, HealthEndpoints, ConnectorEndpoints, CheckEndpoints, FileEndpoints, AuditEndpoints, ReportEndpoints, DashboardEndpoints, RequestEndpoints, MonitorEndpoints, UserEndpoints, type HealthResponse } from './endpoints/index.js';

/** The full EngineR API client: all endpoint groups in one object. */
export interface EnginerClient {
  /** The underlying HTTP client. */
  readonly http: ApiClient;
  /** Entity CRUD + query + merge + retire (docs/06 §6). */
  readonly entities: EntityEndpoints;
  /** Schema draft/publish/retire (docs/06 §5). */
  readonly schemas: SchemaEndpoints;
  /** Security decisions (internal, docs/09). */
  readonly security: SecurityEndpoints;
  /** Health probes (docs/02 §6). */
  readonly health: HealthEndpoints;
  /** Connector CRUD + health check (docs/06 §7, docs/14 §4). */
  readonly connectors: ConnectorEndpoints;
  /** Check + Assessment + Workflow + Task (docs/06 §8, docs/14 §8-10, docs/13 §7-8). */
  readonly checks: CheckEndpoints;
  /** Files: staged upload + download (docs/15 §1-3). */
  readonly files: FileEndpoints;
  /** Audit: query + chain verification + retention (docs/08 §11). */
  readonly audit: AuditEndpoints;
  /** Reports: list + generate (docs/15 §9). */
  readonly reports: ReportEndpoints;
  /** Dashboard summary statistics. */
  readonly dashboard: DashboardEndpoints;
  /** Requests. */
  readonly requests: RequestEndpoints;
  /** Monitoring. */
  readonly monitors: MonitorEndpoints;
  /** User administration + activity (users.manage). */
  readonly users: UserEndpoints;
}

/**
 * Create the full EngineR API client with all endpoint groups.
 *
 * @param config - client configuration (baseUrl, locale, etc.)
 * @returns the typed client.
 */
export function createEnginerClient(config: ApiClientConfig): EnginerClient {
  const http = new ApiClient(config);
  return {
    http,
    entities: new EntityEndpoints(http),
    schemas: new SchemaEndpoints(http),
    security: new SecurityEndpoints(http),
    health: new HealthEndpoints(http),
    connectors: new ConnectorEndpoints(http),
    checks: new CheckEndpoints(http),
    files: new FileEndpoints(http),
    audit: new AuditEndpoints(http),
    reports: new ReportEndpoints(http),
    dashboard: new DashboardEndpoints(http),
    requests: new RequestEndpoints(http),
    monitors: new MonitorEndpoints(http),
    users: new UserEndpoints(http),
  };
}
