/**
 * Tests for endpoint methods: entity CRUD, query, schema, health.
 */

import { describe, it, expect, vi } from 'vitest';
import { createEnginerClient } from '../src/index.js';
import type { ApiClientConfig } from '../src/index.js';

function mockResponse(status: number, body: unknown, headers: Record<string, string> = {}): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json', 'X-Request-Id': 'test', ...headers },
  });
}

function configWithMock(fetchImpl: typeof fetch): ApiClientConfig {
  return { baseUrl: 'http://localhost:8000/api/v1', fetchImpl };
}

describe('EntityEndpoints', () => {
  it('create sends POST /entities with body', async () => {
    const fetchImpl = vi.fn(async () =>
      mockResponse(200, {
        data: { public_id: 'ent-1', entity_id: 1, revision: 1, outcome: 'created' },
        meta: { request_id: 'r' },
      }),
    ) as unknown as typeof fetch;
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.entities.create({
      type_id: 'type-uuid',
      schema_revision: 1,
      identity_mode: 'return_existing',
      statements: [],
    });
    expect(res.data.public_id).toBe('ent-1');
    expect(res.data.outcome).toBe('created');
    expect(fetchImpl).toHaveBeenCalledTimes(1);
    const [url, init] = (fetchImpl as unknown as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toContain('/entities');
    expect(init.method).toBe('POST');
  });

  it('get sends GET /entities/{id}', async () => {
    const fetchImpl = vi.fn(async () =>
      mockResponse(200, {
        data: { public_id: 'ent-1', entity_id: 1, type_id: 't', entity_class: 'data', revision: 1, status: 'active', created_at: '2026-01-01T00:00:00Z' },
        meta: { request_id: 'r' },
      }),
    ) as unknown as typeof fetch;
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.entities.get('ent-1');
    expect(res.data.public_id).toBe('ent-1');
    expect(res.data.status).toBe('active');
  });

  it('query sends POST /entities/query with AST', async () => {
    const fetchImpl = vi.fn(async () =>
      mockResponse(200, {
        data: { rows: [{ public_id: 'ent-1' }], cursor: 'c1' },
        meta: { request_id: 'r' },
      }),
    ) as unknown as typeof fetch;
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.entities.query({
      query: {
        from: { type_id: 't1', alias: 'e' },
        select: [{ field: { system: 'public_id' }, alias: 'id' }],
        sort: [],
        page: { size: 50 },
      },
    });
    expect(res.data.rows).toHaveLength(1);
    expect(res.data.cursor).toBe('c1');
  });

  it('retire sends POST /entities/{id}/retire', async () => {
    const fetchImpl = vi.fn(async () =>
      mockResponse(200, {
        data: { public_id: 'ent-1', revision: 2, status: 'retired' },
        meta: { request_id: 'r' },
      }),
    ) as unknown as typeof fetch;
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.entities.retire('ent-1', { reason: 'duplicate' });
    expect(res.data.status).toBe('retired');
    expect(res.data.revision).toBe(2);
  });

  it('merge sends POST /entities/{id}/merge', async () => {
    const fetchImpl = vi.fn(async () =>
      mockResponse(200, {
        data: { canonical_public_id: 'ent-1', loser_public_id: 'ent-2', canonical_revision: 1, idempotent: false },
        meta: { request_id: 'r' },
      }),
    ) as unknown as typeof fetch;
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.entities.merge('ent-1', { loser_public_id: 'ent-2', reason: 'dup' });
    expect(res.data.canonical_public_id).toBe('ent-1');
    expect(res.data.idempotent).toBe(false);
  });
});

describe('HealthEndpoints', () => {
  it('live sends GET /health/live', async () => {
    const fetchImpl = vi.fn(async () =>
      mockResponse(200, { status: 'ok' }),
    ) as unknown as typeof fetch;
    const api = createEnginerClient({ baseUrl: 'http://localhost:8000', fetchImpl });
    const res = await api.health.live();
    expect(res.data.status).toBe('ok');
  });

  it('ready sends GET /health/ready', async () => {
    const fetchImpl = vi.fn(async () =>
      mockResponse(200, { status: 'ok', dependencies: { db: 'ok' } }),
    ) as unknown as typeof fetch;
    const api = createEnginerClient({ baseUrl: 'http://localhost:8000', fetchImpl });
    const res = await api.health.ready();
    expect(res.data.status).toBe('ok');
    expect(res.data.dependencies?.db).toBe('ok');
  });
});

describe('SchemaEndpoints', () => {
  it('publishDraft sends POST with If-Match', async () => {
    const fetchImpl = vi.fn(async (_url: string, init: RequestInit) => {
      expect(init.headers).toMatchObject({ 'If-Match': 'rev-1' });
      return mockResponse(200, {
        data: { revision_id: 42, state: 'published', types: [], predicates: [] },
        meta: { request_id: 'r' },
      });
    }) as unknown as typeof fetch;
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.schemas.publishDraft('1', 'rev-1', 'initial publish');
    expect(res.data.state).toBe('published');
  });
});
