/**
 * Integration tests for the API key management routes:
 *   POST /api/v1/api-keys   — issue a key
 *   GET  /api/v1/api-keys   — list keys
 *   GET  /api/v1/api-keys/[keyId]    — get key details
 *   DELETE /api/v1/api-keys/[keyId]  — revoke a key
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { NextRequest } from 'next/server';

// ── KV mock ───────────────────────────────────────────────────────────────────

const kvData = new Map<string, string>();

vi.mock('@/lib/kv', () => ({
  kvStore: {
    get: vi.fn(async (key: string) => kvData.get(key) ?? null),
    set: vi.fn(async (key: string, value: string) => { kvData.set(key, value); }),
    del: vi.fn(async (key: string) => { kvData.delete(key); }),
  },
}));

// ── Auth mock — allow internal tier ──────────────────────────────────────────

vi.mock('@/lib/api/auth', () => ({
  authenticateApiRequest: vi.fn(async (_req: NextRequest, tier: string) => {
    const key = _req.headers.get('x-api-key');
    if (key === 'valid-internal-key') return { error: null, apiKey: key };
    const { NextResponse } = await import('next/server');
    return {
      error: NextResponse.json({ error: { status: 401, code: 'UNAUTHORIZED', message: 'Invalid API key', correlationId: 'test' } }, { status: 401 }),
    };
  }),
}));

// ── Rate limit mock — always allow ───────────────────────────────────────────

vi.mock('@/lib/api/rateLimit', () => ({
  applyRateLimit: vi.fn(() => null),
  RATE_LIMIT_PRESETS: { default: {}, publicRead: {} },
}));

// ── Correlation mock ──────────────────────────────────────────────────────────

vi.mock('@/lib/api/correlation', () => ({
  getCorrelationId: vi.fn(() => 'test-correlation-id'),
}));

import { POST, GET } from '../route';
import { GET as getById, DELETE as deleteById } from '../[keyId]/route';

function makeRequest(
  method: string,
  url: string,
  body?: unknown,
  headers: Record<string, string> = {},
): NextRequest {
  const init: RequestInit = {
    method,
    headers: {
      'content-type': 'application/json',
      'x-api-key': 'valid-internal-key',
      ...headers,
    },
  };
  if (body !== undefined) {
    init.body = JSON.stringify(body);
  }
  return new NextRequest(url, init);
}

beforeEach(() => {
  kvData.clear();
  vi.clearAllMocks();
});

// ── POST /api/v1/api-keys ─────────────────────────────────────────────────────

describe('POST /api/v1/api-keys', () => {
  it('issues a key and returns 201 with plaintext key', async () => {
    const req = makeRequest('POST', 'http://localhost/api/v1/api-keys', {
      name: 'Test Partner Key',
      tier: 'partner',
      owner: 'GABC123',
    });

    const res = await POST(req);
    expect(res.status).toBe(201);

    const body = await res.json();
    expect(body.keyId).toMatch(/^kid_/);
    expect(body.key).toMatch(/^sl_partner_/);
    expect(body.name).toBe('Test Partner Key');
    expect(body.tier).toBe('partner');
    expect(body.message).toContain('Store this key securely');
  });

  it('returns 400 for missing required fields', async () => {
    const req = makeRequest('POST', 'http://localhost/api/v1/api-keys', {
      name: 'Missing tier',
      owner: 'GABC',
      // tier is missing
    });

    const res = await POST(req);
    expect(res.status).toBe(400);
    const body = await res.json();
    expect(body.error.code).toBe('VALIDATION_ERROR');
  });

  it('returns 400 for invalid tier', async () => {
    const req = makeRequest('POST', 'http://localhost/api/v1/api-keys', {
      name: 'Bad Tier',
      tier: 'superadmin',
      owner: 'GABC',
    });

    const res = await POST(req);
    expect(res.status).toBe(400);
  });

  it('returns 401 without a valid API key', async () => {
    const req = makeRequest(
      'POST',
      'http://localhost/api/v1/api-keys',
      { name: 'K', tier: 'partner', owner: 'o' },
      { 'x-api-key': 'wrong-key' },
    );

    const res = await POST(req);
    expect(res.status).toBe(401);
  });

  it('returns 400 for invalid JSON body', async () => {
    const req = new NextRequest('http://localhost/api/v1/api-keys', {
      method: 'POST',
      headers: { 'content-type': 'application/json', 'x-api-key': 'valid-internal-key' },
      body: 'not-json',
    });

    const res = await POST(req);
    expect(res.status).toBe(400);
  });
});

// ── GET /api/v1/api-keys ──────────────────────────────────────────────────────

describe('GET /api/v1/api-keys', () => {
  it('returns an empty list when no keys exist', async () => {
    const req = makeRequest('GET', 'http://localhost/api/v1/api-keys');
    const res = await GET(req);
    expect(res.status).toBe(200);
    const body = await res.json();
    expect(body.keys).toEqual([]);
    expect(body.total).toBe(0);
  });

  it('returns issued keys with usage metrics', async () => {
    // Issue a key first
    const postReq = makeRequest('POST', 'http://localhost/api/v1/api-keys', {
      name: 'Listed Key',
      tier: 'partner',
      owner: 'svc',
    });
    await POST(postReq);

    const req = makeRequest('GET', 'http://localhost/api/v1/api-keys');
    const res = await GET(req);
    expect(res.status).toBe(200);

    const body = await res.json();
    expect(body.total).toBe(1);
    expect(body.keys[0].name).toBe('Listed Key');
    expect(body.keys[0].usage).toBeDefined();
    expect(body.keys[0].usage.totalRequests).toBe(0);
  });

  it('returns 401 without a valid API key', async () => {
    const req = makeRequest('GET', 'http://localhost/api/v1/api-keys', undefined, {
      'x-api-key': 'wrong',
    });
    const res = await GET(req);
    expect(res.status).toBe(401);
  });
});

// ── GET /api/v1/api-keys/[keyId] ─────────────────────────────────────────────

describe('GET /api/v1/api-keys/[keyId]', () => {
  it('returns key details for a known keyId', async () => {
    const postReq = makeRequest('POST', 'http://localhost/api/v1/api-keys', {
      name: 'Detail Key',
      tier: 'auditor',
      owner: 'org',
    });
    const postRes = await POST(postReq);
    const { keyId } = await postRes.json();

    const req = makeRequest('GET', `http://localhost/api/v1/api-keys/${keyId}`);
    const res = await getById(req, { params: Promise.resolve({ keyId }) });
    expect(res.status).toBe(200);

    const body = await res.json();
    expect(body.keyId).toBe(keyId);
    expect(body.name).toBe('Detail Key');
    expect(body.tier).toBe('auditor');
  });

  it('returns 404 for an unknown keyId', async () => {
    const req = makeRequest('GET', 'http://localhost/api/v1/api-keys/kid_unknown');
    const res = await getById(req, { params: Promise.resolve({ keyId: 'kid_unknown' }) });
    expect(res.status).toBe(404);
  });
});

// ── DELETE /api/v1/api-keys/[keyId] ──────────────────────────────────────────

describe('DELETE /api/v1/api-keys/[keyId]', () => {
  it('revokes a key and returns 200', async () => {
    const postReq = makeRequest('POST', 'http://localhost/api/v1/api-keys', {
      name: 'Revoke Me',
      tier: 'partner',
      owner: 'svc',
    });
    const postRes = await POST(postReq);
    const { keyId } = await postRes.json();

    const req = makeRequest('DELETE', `http://localhost/api/v1/api-keys/${keyId}`);
    const res = await deleteById(req, { params: Promise.resolve({ keyId }) });
    expect(res.status).toBe(200);

    const body = await res.json();
    expect(body.revoked).toBe(true);
    expect(body.keyId).toBe(keyId);
  });

  it('returns 404 for an unknown keyId', async () => {
    const req = makeRequest('DELETE', 'http://localhost/api/v1/api-keys/kid_unknown');
    const res = await deleteById(req, { params: Promise.resolve({ keyId: 'kid_unknown' }) });
    expect(res.status).toBe(404);
  });
});
