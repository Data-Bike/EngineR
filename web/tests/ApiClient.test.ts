/**
 * Tests for the ApiClient core: envelope parsing, error handling, headers,
 * timeout, and 401 refresh singleflight.
 */

import { describe, it, expect, vi } from 'vitest';
import { ApiClient } from '../src/client/ApiClient.js';
import { ApiClientError } from '../src/types/error.js';

/** Build a mock Response. */
function mockResponse(status: number, body: unknown, headers: Record<string, string> = {}): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: {
      'Content-Type': 'application/json',
      'X-Request-Id': 'test-req-id',
      ...headers,
    },
  });
}

/** Build a mock fetch that returns the given response sequence. */
function mockFetch(responses: Response[]): typeof fetch {
  let i = 0;
  return vi.fn(async (): Promise<Response> => {
    const r = responses[i] ?? responses[responses.length - 1];
    i++;
    // Clone so the body can be read multiple times
    return r.clone();
  }) as unknown as typeof fetch;
}

describe('ApiClient', () => {
  it('parses success envelope', async () => {
    const fetchImpl = mockFetch([
      mockResponse(200, {
        data: { public_id: 'abc' },
        meta: { request_id: 'r1', schema_revision: '42' },
        links: { next: 'cursor1' },
      }),
    ]);
    const client = new ApiClient({ baseUrl: 'http://localhost:8000/api/v1', fetchImpl });
    const res = await client.get<{ public_id: string }>('/entities/abc');
    expect(res.data.public_id).toBe('abc');
    expect(res.meta.request_id).toBe('r1');
    expect(res.meta.schema_revision).toBe('42');
    expect(res.links?.next).toBe('cursor1');
    expect(res.statusCode).toBe(200);
  });

  it('throws typed ApiClientError on error response', async () => {
    const fetchImpl = mockFetch([
      mockResponse(404, {
        error: {
          code: 'not_found',
          message_key: 'errors.not_found',
          request_id: 'r2',
        },
      }),
    ]);
    const client = new ApiClient({ baseUrl: 'http://localhost:8000/api/v1', fetchImpl });
    try {
      await client.get('/entities/missing');
      expect.fail('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(ApiClientError);
      const err = e as ApiClientError;
      expect(err.code).toBe('not_found');
      expect(err.statusCode).toBe(404);
      expect(err.requestId).toBe('r2');
      expect(err.messageKey).toBe('errors.not_found');
    }
  });

  it('sends X-Request-Id, Content-Type, and Accept headers', async () => {
    let capturedInit: RequestInit | null = null;
    const fetchImpl = vi.fn(async (_url: string, init: RequestInit) => {
      capturedInit = init;
      return mockResponse(200, { data: {}, meta: { request_id: 'r' } });
    }) as unknown as typeof fetch;
    const client = new ApiClient({ baseUrl: 'http://localhost:8000/api/v1', fetchImpl });
    await client.get('/test');
    const headers = (capturedInit as RequestInit | null)?.headers as Record<string, string>;
    expect(headers['Accept']).toBe('application/json');
    expect(headers['Content-Type']).toBe('application/json');
    expect(headers['X-Request-Id']).toBeTruthy();
  });

  it('sends locale and timezone headers when configured', async () => {
    let capturedInit: RequestInit | null = null;
    const fetchImpl = vi.fn(async (_url: string, init: RequestInit) => {
      capturedInit = init;
      return mockResponse(200, { data: {}, meta: { request_id: 'r' } });
    }) as unknown as typeof fetch;
    const client = new ApiClient({
      baseUrl: 'http://localhost:8000/api/v1',
      fetchImpl,
      locale: 'ru-RU',
      timezone: 'Europe/Moscow',
    });
    await client.get('/test');
    const headers = (capturedInit as RequestInit | null)?.headers as Record<string, string>;
    expect(headers['X-Locale']).toBe('ru-RU');
    expect(headers['X-Timezone']).toBe('Europe/Moscow');
  });

  it('sends If-Match header for optimistic concurrency', async () => {
    let capturedInit: RequestInit | null = null;
    const fetchImpl = vi.fn(async (_url: string, init: RequestInit) => {
      capturedInit = init;
      return mockResponse(200, { data: {}, meta: { request_id: 'r' } });
    }) as unknown as typeof fetch;
    const client = new ApiClient({ baseUrl: 'http://localhost:8000/api/v1', fetchImpl });
    await client.put('/schemas/drafts/1', { foo: 'bar' }, { ifMatch: 'entity:abc:1' });
    const headers = (capturedInit as RequestInit | null)?.headers as Record<string, string>;
    expect(headers['If-Match']).toBe('entity:abc:1');
  });

  it('sends Idempotency-Key for safe retry of mutations', async () => {
    let capturedInit: RequestInit | null = null;
    const fetchImpl = vi.fn(async (_url: string, init: RequestInit) => {
      capturedInit = init;
      return mockResponse(200, { data: {}, meta: { request_id: 'r' } });
    }) as unknown as typeof fetch;
    const client = new ApiClient({ baseUrl: 'http://localhost:8000/api/v1', fetchImpl });
    await client.post('/entities', { type_id: 'x' }, { idempotencyKey: 'idem-1' });
    const headers = (capturedInit as RequestInit | null)?.headers as Record<string, string>;
    expect(headers['Idempotency-Key']).toBe('idem-1');
  });

  it('sends CSRF token for non-GET requests', async () => {
    let capturedInit: RequestInit | null = null;
    const fetchImpl = vi.fn(async (_url: string, init: RequestInit) => {
      capturedInit = init;
      return mockResponse(200, { data: {}, meta: { request_id: 'r' } });
    }) as unknown as typeof fetch;
    const client = new ApiClient({
      baseUrl: 'http://localhost:8000/api/v1',
      fetchImpl,
      csrfToken: 'csrf-token-123',
    });
    await client.post('/entities', {});
    const headers = (capturedInit as RequestInit | null)?.headers as Record<string, string>;
    expect(headers['X-CSRF-Token']).toBe('csrf-token-123');
  });

  it('does not send CSRF token for GET requests', async () => {
    let capturedInit: RequestInit | null = null;
    const fetchImpl = vi.fn(async (_url: string, init: RequestInit) => {
      capturedInit = init;
      return mockResponse(200, { data: {}, meta: { request_id: 'r' } });
    }) as unknown as typeof fetch;
    const client = new ApiClient({
      baseUrl: 'http://localhost:8000/api/v1',
      fetchImpl,
      csrfToken: 'csrf-token-123',
    });
    await client.get('/entities');
    const headers = (capturedInit as RequestInit | null)?.headers as Record<string, string>;
    expect(headers['X-CSRF-Token']).toBeUndefined();
  });

  it('retries once after 401 refresh singleflight', async () => {
    const fetchImpl = mockFetch([
      mockResponse(401, {
        error: { code: 'unauthorized', message_key: 'errors.unauthorized', request_id: 'r' },
      }),
      mockResponse(200, { data: { ok: true }, meta: { request_id: 'r' } }),
    ]);
    let refreshCalled = 0;
    const client = new ApiClient({
      baseUrl: 'http://localhost:8000/api/v1',
      fetchImpl,
      onRefresh: async () => {
        refreshCalled++;
      },
    });
    const res = await client.get<{ ok: boolean }>('/me');
    expect(res.data.ok).toBe(true);
    expect(refreshCalled).toBe(1);
  });

  it('refresh is singleflight — concurrent 401s share one refresh', async () => {
    // Each call gets 401 first, then 200 after refresh.
    let refreshDone = false;
    const fetchImpl = vi.fn(async () => {
      if (!refreshDone) {
        return mockResponse(401, {
          error: { code: 'unauthorized', message_key: 'errors.unauthorized', request_id: 'r' },
        });
      }
      return mockResponse(200, { data: { ok: true }, meta: { request_id: 'r' } });
    }) as unknown as typeof fetch;
    let refreshCalled = 0;
    const client = new ApiClient({
      baseUrl: 'http://localhost:8000/api/v1',
      fetchImpl,
      onRefresh: async () => {
        refreshCalled++;
        await new Promise((r) => setTimeout(r, 10));
        refreshDone = true;
      },
    });
    const [r1, r2] = await Promise.all([
      client.get<{ ok: boolean }>('/me'),
      client.get<{ ok: boolean }>('/me'),
    ]);
    expect(r1.data.ok).toBe(true);
    expect(r2.data.ok).toBe(true);
    // Both should share the same refresh (singleflight).
    expect(refreshCalled).toBe(1);
  });

  it('handles non-envelope responses (e.g. health)', async () => {
    const fetchImpl = mockFetch([
      mockResponse(200, { status: 'ok' }),
    ]);
    const client = new ApiClient({ baseUrl: 'http://localhost:8000', fetchImpl });
    const res = await client.get<{ status: string }>('/health/live');
    expect(res.data.status).toBe('ok');
    expect(res.statusCode).toBe(200);
  });

  it('wraps unknown error shapes generically', async () => {
    const fetchImpl = mockFetch([
      mockResponse(500, 'Internal Server Error'),
    ]);
    const client = new ApiClient({ baseUrl: 'http://localhost:8000/api/v1', fetchImpl });
    try {
      await client.get('/test');
      expect.fail('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(ApiClientError);
      const err = e as ApiClientError;
      expect(err.code).toBe('internal_error');
      expect(err.statusCode).toBe(500);
    }
  });

  it('builds query parameters correctly', async () => {
    let capturedUrl = '';
    const fetchImpl = vi.fn(async (url: string) => {
      capturedUrl = url;
      return mockResponse(200, { data: [], meta: { request_id: 'r' } });
    }) as unknown as typeof fetch;
    const client = new ApiClient({ baseUrl: 'http://localhost:8000/api/v1', fetchImpl });
    await client.get('/entities', { query: { page_size: 50, cursor: 'abc', include: undefined } });
    expect(capturedUrl).toContain('page_size=50');
    expect(capturedUrl).toContain('cursor=abc');
    expect(capturedUrl).not.toContain('include=');
  });
});

