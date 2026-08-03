/**
 * Stable API error codes and error body (docs/06 §2, docs/07 §13, docs/09 §10).
 *
 * Mirrors `enginer-contracts::error`.
 */

/** Stable, normative error code (wire string form). */
export type ApiErrorCode =
  // Generic
  | 'bad_request'
  | 'unauthorized'
  | 'forbidden'
  | 'not_found'
  | 'conflict'
  | 'precondition_failed'
  | 'unprocessable_entity'
  | 'too_many_requests'
  | 'internal_error'
  | 'service_unavailable'
  // Domain
  | 'entity.version_conflict'
  | 'identity.conflict'
  | 'idempotency.conflict'
  // Schema
  | 'schema.state_invalid'
  | 'schema.validation_failed'
  // Query
  | 'query.syntax_invalid'
  | 'query.unknown_type'
  | 'query.unknown_predicate'
  | 'query.operator_not_supported'
  | 'query.field_forbidden'
  | 'query.too_complex'
  | 'query.requires_batch'
  | 'query.cursor_invalid'
  | 'query.cursor_stale'
  | 'query.projection_lag'
  | 'query.timeout'
  // Security
  | 'security.unauthenticated'
  | 'security.capability_denied'
  | 'security.data_denied'
  | 'security.step_up_required'
  | 'security.policy_stale'
  | 'security.pdp_unavailable'
  | 'security.context_invalid';

/** HTTP status code for an error code (docs/06 §3). */
export function httpStatusForCode(code: ApiErrorCode): number {
  switch (code) {
    case 'bad_request':
      return 400;
    case 'unauthorized':
    case 'security.unauthenticated':
      return 401;
    case 'forbidden':
    case 'security.capability_denied':
    case 'security.step_up_required':
      return 403;
    case 'not_found':
    case 'security.data_denied':
      return 404;
    case 'conflict':
    case 'identity.conflict':
    case 'idempotency.conflict':
    case 'query.cursor_stale':
    case 'security.policy_stale':
      return 409;
    case 'precondition_failed':
    case 'entity.version_conflict':
      return 412;
    case 'unprocessable_entity':
    case 'schema.state_invalid':
    case 'schema.validation_failed':
    case 'query.syntax_invalid':
    case 'query.unknown_type':
    case 'query.unknown_predicate':
    case 'query.operator_not_supported':
    case 'query.field_forbidden':
    case 'query.cursor_invalid':
    case 'query.too_complex':
    case 'query.requires_batch':
    case 'security.context_invalid':
      return 422;
    case 'too_many_requests':
      return 429;
    case 'query.projection_lag':
    case 'security.pdp_unavailable':
    case 'service_unavailable':
      return 503;
    case 'query.timeout':
      return 504;
    case 'internal_error':
    default:
      return 500;
  }
}

/** API error body (docs/06 §2). */
export interface ApiError {
  code: ApiErrorCode;
  message_key: string;
  parameters?: Record<string, unknown>;
  request_id: string;
  message?: string;
}

/** Typed error thrown by the API client. */
export class ApiClientError extends Error {
  readonly code: ApiErrorCode;
  readonly requestId: string;
  readonly statusCode: number;
  readonly parameters?: Record<string, unknown>;
  readonly messageKey: string;

  constructor(error: ApiError, statusCode: number) {
    super(error.message ?? error.message_key);
    this.name = 'ApiClientError';
    this.code = error.code;
    this.requestId = error.request_id;
    this.statusCode = statusCode;
    this.parameters = error.parameters;
    this.messageKey = error.message_key;
  }
}
