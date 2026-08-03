/**
 * Tests for type-level correctness and error code mapping.
 */

import { describe, it, expect } from 'vitest';
import { httpStatusForCode } from '../src/types/error.js';

describe('Error code mapping', () => {
  it('maps generic codes to correct HTTP status', () => {
    expect(httpStatusForCode('bad_request')).toBe(400);
    expect(httpStatusForCode('unauthorized')).toBe(401);
    expect(httpStatusForCode('forbidden')).toBe(403);
    expect(httpStatusForCode('not_found')).toBe(404);
    expect(httpStatusForCode('conflict')).toBe(409);
    expect(httpStatusForCode('precondition_failed')).toBe(412);
    expect(httpStatusForCode('too_many_requests')).toBe(429);
    expect(httpStatusForCode('internal_error')).toBe(500);
    expect(httpStatusForCode('service_unavailable')).toBe(503);
  });

  it('maps domain codes to correct HTTP status', () => {
    expect(httpStatusForCode('entity.version_conflict')).toBe(412);
    expect(httpStatusForCode('identity.conflict')).toBe(409);
    expect(httpStatusForCode('idempotency.conflict')).toBe(409);
  });

  it('maps schema codes to 422', () => {
    expect(httpStatusForCode('schema.state_invalid')).toBe(422);
    expect(httpStatusForCode('schema.validation_failed')).toBe(422);
  });

  it('maps query codes to correct HTTP status', () => {
    expect(httpStatusForCode('query.syntax_invalid')).toBe(422);
    expect(httpStatusForCode('query.unknown_type')).toBe(422);
    expect(httpStatusForCode('query.field_forbidden')).toBe(422);
    expect(httpStatusForCode('query.too_complex')).toBe(422);
    expect(httpStatusForCode('query.requires_batch')).toBe(422);
    expect(httpStatusForCode('query.cursor_stale')).toBe(409);
    expect(httpStatusForCode('query.projection_lag')).toBe(503);
    expect(httpStatusForCode('query.timeout')).toBe(504);
  });

  it('maps security codes to correct HTTP status', () => {
    expect(httpStatusForCode('security.unauthenticated')).toBe(401);
    expect(httpStatusForCode('security.capability_denied')).toBe(403);
    expect(httpStatusForCode('security.data_denied')).toBe(404);
    expect(httpStatusForCode('security.step_up_required')).toBe(403);
    expect(httpStatusForCode('security.policy_stale')).toBe(409);
    expect(httpStatusForCode('security.pdp_unavailable')).toBe(503);
    expect(httpStatusForCode('security.context_invalid')).toBe(422);
  });
});
