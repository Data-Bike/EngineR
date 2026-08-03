/**
 * Tests for new Phase 5 API client endpoints (connector, check, file, audit).
 */
import { describe, it, expect, vi } from 'vitest';
import { createEnginerClient } from '../src/index.js';
import type { ApiClientConfig } from '../src/index.js';
import {
  TrustRank,
  CheckKind,
  CheckStatus,
  RiskLevel,
  TaskStatus,
  TaskPriority,
  DecisionKind,
  UploadStatus,
  StorageClass,
  AuditKind,
} from '../src/types/index.js';

function mockResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json', 'X-Request-Id': 'test' },
  });
}

function configWithMock(fetchImpl: typeof fetch): ApiClientConfig {
  return { baseUrl: 'http://localhost:8000/api/v1', fetchImpl };
}

function makeFetch(body: unknown, status = 200): typeof fetch {
  return vi.fn(async () => mockResponse(status, body)) as unknown as typeof fetch;
}

describe('ConnectorEndpoints', () => {
  it('list connectors', async () => {
    const fetchImpl = makeFetch({
      data: [{ instance_id: 'c1', connector_type_id: 'fns', tenant_id: 't1', enabled: true, environment: 'prod', rate_limit_rps: 10, trust_rank: 75 }],
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.connectors.list();
    expect(res.data).toHaveLength(1);
    expect(res.data[0].instance_id).toBe('c1');
  });

  it('get connector', async () => {
    const fetchImpl = makeFetch({
      data: { instance_id: 'c1', connector_type_id: 'fns', tenant_id: 't1', enabled: true, environment: 'prod', rate_limit_rps: 10, trust_rank: 75 },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.connectors.get('c1');
    expect(res.data.instance_id).toBe('c1');
  });

  it('create connector', async () => {
    const fetchImpl = makeFetch({
      data: { instance_id: 'c1', connector_type_id: 'fns', tenant_id: 't1', enabled: true, environment: 'prod', rate_limit_rps: 10, trust_rank: 75 },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.connectors.create({ connector_type_id: 'fns', environment: 'prod' });
    expect(res.data.instance_id).toBe('c1');
  });

  it('update connector', async () => {
    const fetchImpl = makeFetch({
      data: { instance_id: 'c1', connector_type_id: 'fns', tenant_id: 't1', enabled: false, environment: 'prod', rate_limit_rps: 20, trust_rank: 75 },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.connectors.update('c1', { enabled: false, rate_limit_rps: 20 });
    expect(res.data.enabled).toBe(false);
  });

  it('enable connector', async () => {
    const fetchImpl = makeFetch({ data: { ok: true }, meta: {} });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.connectors.enable('c1');
    expect(res.data).toBeDefined();
  });

  it('disable connector', async () => {
    const fetchImpl = makeFetch({ data: { ok: true }, meta: {} });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.connectors.disable('c1');
    expect(res.data).toBeDefined();
  });

  it('health check', async () => {
    const fetchImpl = makeFetch({
      data: { healthy: true, latency_ms: 42, message: 'ok' },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.connectors.healthCheck('c1');
    expect(res.data.healthy).toBe(true);
    expect(res.data.latency_ms).toBe(42);
  });

  it('delete connector', async () => {
    const fetchImpl = makeFetch({ data: null, meta: {} });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.connectors.delete('c1');
    expect(res.data).toBeNull();
  });
});

describe('CheckEndpoints', () => {
  it('request check', async () => {
    const fetchImpl = makeFetch({
      data: { check_id: 'chk-1', status: 'pending', created_at: '2026-01-01T00:00:00Z' },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.checks.requestCheck({
      target_entity_id: 'ent-1',
      check_kind: 'kyc',
      parameters: {},
    });
    expect(res.data.check_id).toBe('chk-1');
  });

  it('get check', async () => {
    const fetchImpl = makeFetch({
      data: { check_id: 'chk-1', status: 'completed', created_at: '2026-01-01T00:00:00Z' },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.checks.getCheck('chk-1');
    expect(res.data.status).toBe('completed');
  });

  it('get check evidence', async () => {
    const fetchImpl = makeFetch({
      data: { evidence: [{ type: 'document', hash: 'abc' }] },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.checks.getCheckEvidence('chk-1');
    expect(res.data.evidence).toHaveLength(1);
  });

  it('get assessment', async () => {
    const fetchImpl = makeFetch({
      data: {
        assessment_id: 'as-1', target_entity_id: 'ent-1', final_score: 75.5,
        level: 'high', status: 'approved', generated_at: '2026-01-01T00:00:00Z', factors: [],
      },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.checks.getAssessment('ent-1');
    expect(res.data.final_score).toBe(75.5);
    expect(res.data.level).toBe('high');
  });

  it('trigger assessment', async () => {
    const fetchImpl = makeFetch({
      data: {
        assessment_id: 'as-2', target_entity_id: 'ent-1', final_score: 0,
        level: 'minimal', status: 'draft', generated_at: '2026-01-01T00:00:00Z', factors: [],
      },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.checks.triggerAssessment('ent-1', { force: true });
    expect(res.data.assessment_id).toBe('as-2');
  });

  it('get factors', async () => {
    const fetchImpl = makeFetch({
      data: [{ factor_id: 'f1', criterion_id: 'c1', severity: 'high', status: 'active' }],
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.checks.getFactors('ent-1');
    expect(res.data).toHaveLength(1);
    expect(res.data[0].severity).toBe('high');
  });

  it('start workflow', async () => {
    const fetchImpl = makeFetch({
      data: { instance_id: 'wf-1', definition_id: 'request_processing', current_state: 'initial', terminated: false },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.checks.startWorkflow({
      definition_id: 'request_processing', target_entity_id: 'ent-1', variables: {},
    });
    expect(res.data.instance_id).toBe('wf-1');
  });

  it('get workflow', async () => {
    const fetchImpl = makeFetch({
      data: { instance_id: 'wf-1', definition_id: 'request_processing', current_state: 'review', terminated: false },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.checks.getWorkflow('wf-1');
    expect(res.data.current_state).toBe('review');
  });

  it('transition workflow', async () => {
    const fetchImpl = makeFetch({
      data: { instance_id: 'wf-1', definition_id: 'request_processing', current_state: 'approved', terminated: false },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.checks.transitionWorkflow('wf-1', { to: 'approved', trigger: 'approve' });
    expect(res.data.current_state).toBe('approved');
  });

  it('list workflow tasks', async () => {
    const fetchImpl = makeFetch({
      data: [{ task_id: 't1', title: 'Review', kind: 'review', status: 'pending', priority: 'high', assignee_id: null, due_date: '2026-01-04T00:00:00Z' }],
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.checks.listWorkflowTasks('wf-1');
    expect(res.data).toHaveLength(1);
  });

  it('complete task', async () => {
    const fetchImpl = makeFetch({
      data: { task_id: 't1', title: 'Review', kind: 'review', status: 'completed', priority: 'high', assignee_id: 'u1', due_date: '2026-01-04T00:00:00Z' },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.checks.completeTask('t1', { result: { approved: true } });
    expect(res.data.status).toBe('completed');
  });

  it('cancel task', async () => {
    const fetchImpl = makeFetch({
      data: { task_id: 't1', title: 'Review', kind: 'review', status: 'cancelled', priority: 'high', assignee_id: null, due_date: '2026-01-04T00:00:00Z' },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.checks.cancelTask('t1', { reason: 'not needed' });
    expect(res.data.status).toBe('cancelled');
  });
});

describe('FileEndpoints', () => {
  it('upload init', async () => {
    const fetchImpl = makeFetch({
      data: { upload_id: 'up-1', total_chunks: 5, status: 'initiated' },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.files.uploadInit({
      filename: 'test.xlsx', content_type: 'application/vnd.openxmlformats',
      expected_size: 1024, expected_hash: 'abc123', hash_algorithm: 'blake2b', chunk_size: 256,
    });
    expect(res.data.upload_id).toBe('up-1');
    expect(res.data.total_chunks).toBe(5);
  });

  it('upload complete', async () => {
    const fetchImpl = makeFetch({
      data: { upload_id: 'up-1', status: 'completed', progress: 1.0, received_chunks: 5, total_chunks: 5, object_id: 'obj-1' },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.files.uploadComplete('up-1', { final_hash: 'abc123', total_size: 1024 });
    expect(res.data.status).toBe('completed');
    expect(res.data.object_id).toBe('obj-1');
  });

  it('upload abort', async () => {
    const fetchImpl = makeFetch({ data: null, meta: {} });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.files.uploadAbort('up-1', { reason: 'cancelled' });
    expect(res.data).toBeNull();
  });

  it('upload status', async () => {
    const fetchImpl = makeFetch({
      data: { upload_id: 'up-1', status: 'uploading', progress: 0.4, received_chunks: 2, total_chunks: 5, object_id: null },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.files.uploadStatus('up-1');
    expect(res.data.progress).toBe(0.4);
  });

  it('get file metadata', async () => {
    const fetchImpl = makeFetch({
      data: {
        object_id: 'obj-1', filename: 'test.xlsx', content_type: 'application/vnd.openxmlformats',
        size: 1024, content_hash: 'abc123', created_at: '2026-01-01T00:00:00Z', classification: 'internal',
      },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.files.getFile('obj-1');
    expect(res.data.filename).toBe('test.xlsx');
    expect(res.data.classification).toBe('internal');
  });

  it('delete file', async () => {
    const fetchImpl = makeFetch({ data: null, meta: {} });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.files.deleteFile('obj-1');
    expect(res.data).toBeNull();
  });
});

describe('AuditEndpoints', () => {
  it('query audit entries', async () => {
    const fetchImpl = makeFetch({
      data: [{
        audit_id: 1, sequence: 0, previous_hash: '0000', entry_hash: 'abc',
        tenant_id: 't1', occurred_at: '2026-01-01T00:00:00Z', actor: 'user-1',
        effective_actor: null, kind: 'decision', action: 'entity.read',
        resource_kind: 'entity', resource_id: 'ent-1', effect: 'allow',
        reason_ids: [], policy_revision: null, request_id: null,
        correlation_id: null, query_fingerprint: null, row_count: null,
        bytes: null, client: null, service: null, metadata: {},
      }],
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.audit.query({ kind: 'decision', limit: 10 });
    expect(res.data).toHaveLength(1);
    expect(res.data[0].kind).toBe('decision');
  });

  it('latest checkpoint', async () => {
    const fetchImpl = makeFetch({
      data: { checkpoint_id: 1, tenant_id: 't1', sequence: 100, entry_hash: 'abc', created_at: '2026-01-01T00:00:00Z' },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.audit.latestCheckpoint();
    expect(res.data?.sequence).toBe(100);
  });

  it('create checkpoint', async () => {
    const fetchImpl = makeFetch({
      data: { checkpoint_id: 2, tenant_id: 't1', sequence: 150, entry_hash: 'def', created_at: '2026-01-02T00:00:00Z' },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.audit.createCheckpoint();
    expect(res.data.sequence).toBe(150);
  });

  it('verify chain', async () => {
    const fetchImpl = makeFetch({
      data: { valid: true, entry_count: 100, last_sequence: 99, last_hash: 'abc' },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.audit.verifyChain();
    expect(res.data.valid).toBe(true);
  });

  it('get retention policy', async () => {
    const fetchImpl = makeFetch({
      data: { tenant_id: 't1', retention_days: 365, legal_hold: false, archive_before_delete: true, archive_url: null },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.audit.getRetentionPolicy();
    expect(res.data.retention_days).toBe(365);
  });

  it('set retention policy', async () => {
    const fetchImpl = makeFetch({
      data: { tenant_id: 't1', retention_days: 730, legal_hold: true, archive_before_delete: true, archive_url: 's3://archive' },
      meta: {},
    });
    const api = createEnginerClient(configWithMock(fetchImpl));
    const res = await api.audit.setRetentionPolicy({ retention_days: 730, legal_hold: true });
    expect(res.data.retention_days).toBe(730);
    expect(res.data.legal_hold).toBe(true);
  });
});

// --- Type/const tests ---

describe('Type constants', () => {
  it('TrustRank values', () => {
    expect(TrustRank.Untrusted).toBe(0);
    expect(TrustRank.Authoritative).toBe(100);
  });

  it('CheckKind values', () => {
    expect(CheckKind.Kyc).toBe('kyc');
    expect(CheckKind.Sanctions).toBe('sanctions');
  });

  it('CheckStatus values', () => {
    expect(CheckStatus.Pending).toBe('pending');
    expect(CheckStatus.Completed).toBe('completed');
  });

  it('RiskLevel values', () => {
    expect(RiskLevel.Minimal).toBe('minimal');
    expect(RiskLevel.Critical).toBe('critical');
  });

  it('TaskStatus values', () => {
    expect(TaskStatus.Pending).toBe('pending');
    expect(TaskStatus.Escalated).toBe('escalated');
  });

  it('TaskPriority values', () => {
    expect(TaskPriority.Low).toBe('low');
    expect(TaskPriority.Urgent).toBe('urgent');
  });

  it('DecisionKind values', () => {
    expect(DecisionKind.Approve).toBe('approve');
    expect(DecisionKind.Escalate).toBe('escalate');
  });

  it('UploadStatus values', () => {
    expect(UploadStatus.Initiated).toBe('initiated');
    expect(UploadStatus.Completed).toBe('completed');
  });

  it('StorageClass values', () => {
    expect(StorageClass.Hot).toBe('hot');
    expect(StorageClass.Archive).toBe('archive');
  });

  it('AuditKind values', () => {
    expect(AuditKind.Authentication).toBe('authentication');
    expect(AuditKind.BreakGlass).toBe('break_glass');
    expect(AuditKind.Impersonation).toBe('impersonation');
  });
});
