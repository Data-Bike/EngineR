import { describe, it, expect } from 'vitest';
import { httpStatusForCode } from '../../web/src/types/error.js';

describe('Error code mapping (shared with API client)', () => {
  it('maps generic codes', () => {
    expect(httpStatusForCode('bad_request')).toBe(400);
    expect(httpStatusForCode('unauthorized')).toBe(401);
    expect(httpStatusForCode('forbidden')).toBe(403);
    expect(httpStatusForCode('not_found')).toBe(404);
    expect(httpStatusForCode('internal_error')).toBe(500);
  });

  it('maps domain codes', () => {
    expect(httpStatusForCode('entity.version_conflict')).toBe(412);
    expect(httpStatusForCode('identity.conflict')).toBe(409);
  });

  it('maps security codes', () => {
    expect(httpStatusForCode('security.unauthenticated')).toBe(401);
    expect(httpStatusForCode('security.capability_denied')).toBe(403);
    expect(httpStatusForCode('security.data_denied')).toBe(404);
    expect(httpStatusForCode('security.step_up_required')).toBe(403);
  });

  it('maps query codes', () => {
    expect(httpStatusForCode('query.timeout')).toBe(504);
    expect(httpStatusForCode('query.projection_lag')).toBe(503);
    expect(httpStatusForCode('query.syntax_invalid')).toBe(422);
  });
});
