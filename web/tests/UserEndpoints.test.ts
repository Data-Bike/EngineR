/**
 * UserEndpoints tests: user administration API surface (users.manage).
 */

import { describe, it, expect, vi } from 'vitest';
import { createEnginerClient } from '../src/index.js';
import type { ApiClientConfig } from '../src/index.js';

function mockResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

function client(fetchImpl: typeof fetch) {
  const cfg: ApiClientConfig = { baseUrl: 'http://x/api/v1', fetchImpl };
  return createEnginerClient(cfg);
}

describe('UserEndpoints', () => {
  it('list serializes filters: q, group, inherited, ip, status', async () => {
    const fetchImpl = vi.fn(async () => mockResponse(200, { data: [], meta: {} })) as unknown as typeof fetch;
    const api = client(fetchImpl);
    await api.users.list({ q: 'ivan', group_id: 'g1', inherited: true, ip: '10.', status: 'active' });
    const [url] = (fetchImpl as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toContain('/users?');
    expect(url).toContain('q=ivan');
    expect(url).toContain('group_id=g1');
    expect(url).toContain('inherited=1');
    expect(url).toContain('ip=10.');
    expect(url).toContain('status=active');
  });

  it('list omits inherited=false', async () => {
    const fetchImpl = vi.fn(async () => mockResponse(200, { data: [], meta: {} })) as unknown as typeof fetch;
    const api = client(fetchImpl);
    await api.users.list({ inherited: false });
    const [url] = (fetchImpl as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).not.toContain('inherited');
  });

  it('listRoleGroups hits /users/groups', async () => {
    const fetchImpl = vi.fn(async () =>
      mockResponse(200, { data: [{ id: 'g', name: 'admins' }], meta: {} }),
    ) as unknown as typeof fetch;
    const api = client(fetchImpl);
    const res = await api.users.listRoleGroups();
    const [url] = (fetchImpl as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toContain('/users/groups');
    expect((res.data as any[])[0].name).toBe('admins');
  });

  it('create posts login/display_name/password/groups', async () => {
    const fetchImpl = vi.fn(async () =>
      mockResponse(200, { data: { principal_id: 'p9' }, meta: {} }),
    ) as unknown as typeof fetch;
    const api = client(fetchImpl);
    await api.users.create({
      login: 'bob',
      display_name: 'Bob',
      password: 's3cret-pass',
      groups: ['admins'],
    });
    const [url, init] = (fetchImpl as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toContain('/users');
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body as string)).toMatchObject({ login: 'bob' });
  });

  it('update sends PUT, remove sends DELETE', async () => {
    const calls: [string, any][] = [];
    const fetchImpl = vi.fn(async (u: any, i: any) => {
      calls.push([u, i]);
      return mockResponse(200, { data: {}, meta: {} });
    }) as unknown as typeof fetch;
    const api = client(fetchImpl);
    await api.users.update('p1', { display_name: 'New', status: 'disabled' });
    await api.users.remove('p1');
    expect(calls[0][0]).toContain('/users/p1');
    expect(calls[0][1].method).toBe('PUT');
    expect(calls[1][1].method).toBe('DELETE');
  });

  it('activity passes free-text q', async () => {
    const fetchImpl = vi.fn(async () =>
      mockResponse(200, { data: { created: [], changed: [], viewed: [], log: [] }, meta: {} }),
    ) as unknown as typeof fetch;
    const api = client(fetchImpl);
    await api.users.activity('p1', 'login');
    const [url] = (fetchImpl as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toContain('/users/p1/activity?q=login');
  });
});
