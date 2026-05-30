/**
 * Tests for the API key registry:
 *   - Key issuance (format, hashing, storage)
 *   - Authentication (valid key, revoked key, expired key, wrong key)
 *   - Revocation (idempotent, audit trail preserved)
 *   - Usage tracking (counters, window reset, limit enforcement)
 *   - Listing
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';

// ── KV mock ───────────────────────────────────────────────────────────────────

const kvData = new Map<string, string>();

vi.mock('@/lib/kv', () => ({
  kvStore: {
    get: vi.fn(async (key: string) => kvData.get(key) ?? null),
    set: vi.fn(async (key: string, value: string) => {
      kvData.set(key, value);
    }),
    del: vi.fn(async (key: string) => {
      kvData.delete(key);
    }),
  },
}));

import {
  issueApiKey,
  getApiKeyRecord,
  revokeApiKey,
  listApiKeys,
  authenticateKey,
  trackAndCheckUsage,
  getApiKeyUsage,
  hashApiKey,
  API_KEY_TIER_LIMITS,
} from '../apiKeyRegistry';

beforeEach(() => {
  kvData.clear();
  vi.clearAllMocks();
});

// ── Issuance ──────────────────────────────────────────────────────────────────

describe('issueApiKey', () => {
  it('returns a plaintext key with the correct prefix', async () => {
    const { plaintext } = await issueApiKey({
      name: 'Test Key',
      tier: 'partner',
      owner: 'GABC123',
    });
    expect(plaintext).toMatch(/^sl_partner_[0-9a-f]{64}$/);
  });

  it('stores the record in KV with hashed key', async () => {
    const { record, plaintext } = await issueApiKey({
      name: 'My Key',
      tier: 'internal',
      owner: 'service-a',
    });

    expect(record.keyId).toMatch(/^kid_[0-9a-f]{16}$/);
    expect(record.name).toBe('My Key');
    expect(record.tier).toBe('internal');
    expect(record.owner).toBe('service-a');
    expect(record.revoked).toBe(false);
    expect(record.revokedAt).toBe(0);
    expect(record.createdAt).toBeGreaterThan(0);

    // The stored hash must NOT equal the plaintext
    expect(record.keyHash).not.toBe(plaintext);
    // The hash must match what hashApiKey produces
    const expectedHash = await hashApiKey(plaintext);
    expect(record.keyHash).toBe(expectedHash);
  });

  it('sets expiresAt when expiresInDays is provided', async () => {
    const before = Date.now();
    const { record } = await issueApiKey({
      name: 'Expiring Key',
      tier: 'auditor',
      owner: 'auditor-org',
      expiresInDays: 30,
    });
    const after = Date.now();

    const expectedMin = before + 30 * 24 * 60 * 60 * 1000;
    const expectedMax = after + 30 * 24 * 60 * 60 * 1000;
    expect(record.expiresAt).toBeGreaterThanOrEqual(expectedMin);
    expect(record.expiresAt).toBeLessThanOrEqual(expectedMax);
  });

  it('sets expiresAt to 0 when no expiry is provided', async () => {
    const { record } = await issueApiKey({ name: 'Permanent', tier: 'partner', owner: 'svc' });
    expect(record.expiresAt).toBe(0);
  });

  it('appends keyId to the global list', async () => {
    const { record: r1 } = await issueApiKey({ name: 'K1', tier: 'partner', owner: 'o' });
    const { record: r2 } = await issueApiKey({ name: 'K2', tier: 'partner', owner: 'o' });

    const all = await listApiKeys();
    const ids = all.map((r) => r.keyId);
    expect(ids).toContain(r1.keyId);
    expect(ids).toContain(r2.keyId);
  });

  it('initialises a usage record with zero counters', async () => {
    const { record } = await issueApiKey({ name: 'K', tier: 'partner', owner: 'o' });
    const usage = await getApiKeyUsage(record.keyId);
    expect(usage).not.toBeNull();
    expect(usage!.totalRequests).toBe(0);
    expect(usage!.windowRequests).toBe(0);
  });
});

// ── Authentication ────────────────────────────────────────────────────────────

describe('authenticateKey', () => {
  it('returns the record for a valid key', async () => {
    const { plaintext, record } = await issueApiKey({
      name: 'Auth Test',
      tier: 'partner',
      owner: 'svc',
    });

    const result = await authenticateKey(plaintext);
    expect(result).not.toBeNull();
    expect(result!.keyId).toBe(record.keyId);
  });

  it('returns null for an unknown key', async () => {
    const result = await authenticateKey('sl_partner_' + 'a'.repeat(64));
    expect(result).toBeNull();
  });

  it('returns null for a key with wrong prefix', async () => {
    const result = await authenticateKey('invalid_key_format');
    expect(result).toBeNull();
  });

  it('returns null for a revoked key', async () => {
    const { plaintext, record } = await issueApiKey({
      name: 'Revoke Test',
      tier: 'partner',
      owner: 'svc',
    });
    await revokeApiKey(record.keyId);

    const result = await authenticateKey(plaintext);
    expect(result).toBeNull();
  });

  it('returns null for an expired key', async () => {
    const { plaintext, record } = await issueApiKey({
      name: 'Expired',
      tier: 'partner',
      owner: 'svc',
      expiresInDays: 1,
    });

    // Manually set expiresAt to the past
    const stored = await getApiKeyRecord(record.keyId);
    stored!.expiresAt = Date.now() - 1000;
    const { kvStore } = await import('@/lib/kv');
    await kvStore.set(`apikey:${record.keyId}`, JSON.stringify(stored!), 999);

    const result = await authenticateKey(plaintext);
    expect(result).toBeNull();
  });
});

// ── Revocation ────────────────────────────────────────────────────────────────

describe('revokeApiKey', () => {
  it('marks the key as revoked and sets revokedAt', async () => {
    const { record } = await issueApiKey({ name: 'R', tier: 'partner', owner: 'o' });
    const before = Date.now();
    const result = await revokeApiKey(record.keyId);
    const after = Date.now();

    expect(result).toBe(true);

    const updated = await getApiKeyRecord(record.keyId);
    expect(updated!.revoked).toBe(true);
    expect(updated!.revokedAt).toBeGreaterThanOrEqual(before);
    expect(updated!.revokedAt).toBeLessThanOrEqual(after);
  });

  it('is idempotent — revoking twice returns true', async () => {
    const { record } = await issueApiKey({ name: 'R2', tier: 'partner', owner: 'o' });
    await revokeApiKey(record.keyId);
    const result = await revokeApiKey(record.keyId);
    expect(result).toBe(true);
  });

  it('returns false for a non-existent keyId', async () => {
    const result = await revokeApiKey('kid_nonexistent');
    expect(result).toBe(false);
  });

  it('preserves the record after revocation (audit trail)', async () => {
    const { record } = await issueApiKey({ name: 'Audit', tier: 'partner', owner: 'o' });
    await revokeApiKey(record.keyId);

    const stored = await getApiKeyRecord(record.keyId);
    expect(stored).not.toBeNull();
    expect(stored!.name).toBe('Audit');
  });
});

// ── Usage tracking ────────────────────────────────────────────────────────────

describe('trackAndCheckUsage', () => {
  it('allows requests within the limit', async () => {
    const { record } = await issueApiKey({ name: 'U', tier: 'partner', owner: 'o' });
    const result = await trackAndCheckUsage(record.keyId, 'partner', '/api/test');
    expect(result.allowed).toBe(true);
  });

  it('increments totalRequests and windowRequests', async () => {
    const { record } = await issueApiKey({ name: 'U2', tier: 'partner', owner: 'o' });
    await trackAndCheckUsage(record.keyId, 'partner', '/api/test');
    await trackAndCheckUsage(record.keyId, 'partner', '/api/test');

    const usage = await getApiKeyUsage(record.keyId);
    expect(usage!.totalRequests).toBe(2);
    expect(usage!.windowRequests).toBe(2);
  });

  it('tracks per-endpoint counts', async () => {
    const { record } = await issueApiKey({ name: 'U3', tier: 'partner', owner: 'o' });
    await trackAndCheckUsage(record.keyId, 'partner', '/api/products');
    await trackAndCheckUsage(record.keyId, 'partner', '/api/products');
    await trackAndCheckUsage(record.keyId, 'partner', '/api/events');

    const usage = await getApiKeyUsage(record.keyId);
    expect(usage!.endpointCounts['/api/products']).toBe(2);
    expect(usage!.endpointCounts['/api/events']).toBe(1);
  });

  it('blocks requests when the window limit is exceeded', async () => {
    const { record } = await issueApiKey({ name: 'Limited', tier: 'auditor', owner: 'o' });
    const limit = API_KEY_TIER_LIMITS.auditor.requestsPerWindow;

    // Manually set windowRequests to the limit
    const usage = await getApiKeyUsage(record.keyId);
    usage!.windowRequests = limit;
    usage!.windowStartedAt = Date.now();
    const { kvStore } = await import('@/lib/kv');
    await kvStore.set(`apikey:usage:${record.keyId}`, JSON.stringify(usage!), 999);

    const result = await trackAndCheckUsage(record.keyId, 'auditor', '/api/test');
    expect(result.allowed).toBe(false);
    expect(result.retryAfterSeconds).toBeGreaterThan(0);
  });

  it('resets the window when windowMs has elapsed', async () => {
    const { record } = await issueApiKey({ name: 'Window', tier: 'partner', owner: 'o' });
    const limits = API_KEY_TIER_LIMITS.partner;

    // Set windowStartedAt to the past (window expired)
    const usage = await getApiKeyUsage(record.keyId);
    usage!.windowRequests = limits.requestsPerWindow; // at limit
    usage!.windowStartedAt = Date.now() - limits.windowMs - 1000; // expired
    const { kvStore } = await import('@/lib/kv');
    await kvStore.set(`apikey:usage:${record.keyId}`, JSON.stringify(usage!), 999);

    // Should be allowed because window reset
    const result = await trackAndCheckUsage(record.keyId, 'partner', '/api/test');
    expect(result.allowed).toBe(true);
  });
});

// ── Listing ───────────────────────────────────────────────────────────────────

describe('listApiKeys', () => {
  it('returns an empty array when no keys exist', async () => {
    const keys = await listApiKeys();
    expect(keys).toEqual([]);
  });

  it('returns all issued keys including revoked ones', async () => {
    const { record: r1 } = await issueApiKey({ name: 'A', tier: 'partner', owner: 'o' });
    const { record: r2 } = await issueApiKey({ name: 'B', tier: 'internal', owner: 'o' });
    await revokeApiKey(r1.keyId);

    const keys = await listApiKeys();
    expect(keys).toHaveLength(2);
    const revokedKey = keys.find((k) => k.keyId === r1.keyId);
    expect(revokedKey!.revoked).toBe(true);
    const activeKey = keys.find((k) => k.keyId === r2.keyId);
    expect(activeKey!.revoked).toBe(false);
  });
});
