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
      from: { type_id: 't1', alias: 'e' },
      select: [{ field: { system: 'public_id' }, alias: 'id' }],
      sort: [],
      page: { size: 50 },
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
      expect(JSON.parse(String(init.body))).toEqual({ reason: 'initial publish' });
      return mockResponse(200, {
        data: {
          revision_id: '42', display_name: 'Next', description: '', status: 'published',
          semantic_version: '1.1.0', version: 3, etag: '"schema:42:3"', types: [],
          predicates: [], changes: [], type_count: 0, predicate_count: 0,
          created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z',
        },
        meta: { request_id: 'r' },
      });
    }) as unknown as typeof fetch;
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.schemas.publishDraft('1', 'rev-1', 'initial publish');
    expect(res.data.status).toBe('published');
  });

  it('updateDraft sends typed content with If-Match', async () => {
    const fetchImpl = vi.fn(async (url: string, init: RequestInit) => {
      expect(url).toContain('/schemas/drafts/2');
      expect(init.method).toBe('PUT');
      expect(init.headers).toMatchObject({ 'If-Match': '"schema:2:1"' });
      expect(JSON.parse(String(init.body))).toMatchObject({ semantic_version: '1.1.0' });
      return mockResponse(200, {
        data: {
          revision_id: '2', display_name: 'Draft', description: '', status: 'draft',
          semantic_version: '1.1.0', version: 2, etag: '"schema:2:2"', types: [],
          predicates: [], changes: [], type_count: 0, predicate_count: 0,
          created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z',
        },
        meta: { request_id: 'r' },
      });
    }) as unknown as typeof fetch;
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.schemas.updateDraft('2', {
      display_name: 'Draft',
      description: '',
      semantic_version: '1.1.0',
      types: [],
      predicates: [],
      changes: [],
    }, '"schema:2:1"');
    expect(res.data.version).toBe(2);
  });

  it('retireRevision uses the revision endpoint and reason', async () => {
    const fetchImpl = vi.fn(async (url: string, init: RequestInit) => {
      expect(url).toContain('/schemas/revisions/7/retire');
      expect(init.headers).toMatchObject({ 'If-Match': '"schema:7:4"' });
      expect(JSON.parse(String(init.body))).toEqual({ reason: 'Replaced by revision 8' });
      return mockResponse(200, {
        data: {
          revision_id: '7', display_name: 'Old', description: '', status: 'retired',
          semantic_version: '1.0.0', version: 5, etag: '"schema:7:5"', types: [],
          predicates: [], changes: [], type_count: 0, predicate_count: 0,
          created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z',
        },
        meta: { request_id: 'r' },
      });
    }) as unknown as typeof fetch;
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.schemas.retireRevision('7', 'Replaced by revision 8', '"schema:7:4"');
    expect(res.data.status).toBe('retired');
  });
});
