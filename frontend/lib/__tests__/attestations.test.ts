/**
 * Tests for the attestation registry:
 *   - Adding attestations
 *   - Listing by product and by issuer
 *   - Revocation (issuer-only, idempotent, audit trail)
 *   - Validation (active, revoked, expired, reference mismatch)
 *   - Status computation
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
  addAttestation,
  getAttestation,
  listAttestationsForProduct,
  listAttestationsByIssuer,
  revokeAttestation,
  validateAttestation,
} from '../attestations';

beforeEach(() => {
  kvData.clear();
  vi.clearAllMocks();
});

// ── Helpers ───────────────────────────────────────────────────────────────────

async function makeAttestation(overrides: Partial<Parameters<typeof addAttestation>[0]> = {}) {
  return addAttestation({
    productId: 'prod-001',
    issuerAddress: 'GAUDITOR123',
    issuerName: 'Acme Auditors',
    trustLevel: 'verified',
    attestationType: 'audit',
    summary: 'Annual supply chain audit passed',
    signedReference: 'abc123def456',
    ...overrides,
  });
}

// ── Adding attestations ───────────────────────────────────────────────────────

describe('addAttestation', () => {
  it('creates a record with a unique attestationId', async () => {
    const record = await makeAttestation();
    expect(record.attestationId).toMatch(/^att_[0-9a-f]{24}$/);
  });

  it('stores all provided fields correctly', async () => {
    const record = await makeAttestation({
      productId: 'p-xyz',
      issuerAddress: 'GISSUER',
      issuerName: 'Test Org',
      trustLevel: 'trusted',
      attestationType: 'compliance',
      summary: 'ISO 9001 compliant',
      signedReference: 'ref-hash-001',
      reportUrl: 'https://example.com/report.pdf',
      metadata: '{"standard":"ISO9001"}',
    });

    expect(record.productId).toBe('p-xyz');
    expect(record.issuerAddress).toBe('GISSUER');
    expect(record.issuerName).toBe('Test Org');
    expect(record.trustLevel).toBe('trusted');
    expect(record.attestationType).toBe('compliance');
    expect(record.summary).toBe('ISO 9001 compliant');
    expect(record.signedReference).toBe('ref-hash-001');
    expect(record.reportUrl).toBe('https://example.com/report.pdf');
    expect(record.metadata).toBe('{"standard":"ISO9001"}');
    expect(record.revoked).toBe(false);
    expect(record.revokedAt).toBe(0);
    expect(record.createdAt).toBeGreaterThan(0);
  });

  it('sets expiresAt when expiresInDays is provided', async () => {
    const before = Date.now();
    const record = await makeAttestation({ expiresInDays: 90 });
    const after = Date.now();

    const expectedMin = before + 90 * 24 * 60 * 60 * 1000;
    const expectedMax = after + 90 * 24 * 60 * 60 * 1000;
    expect(record.expiresAt).toBeGreaterThanOrEqual(expectedMin);
    expect(record.expiresAt).toBeLessThanOrEqual(expectedMax);
  });

  it('sets expiresAt to 0 when no expiry is provided', async () => {
    const record = await makeAttestation();
    expect(record.expiresAt).toBe(0);
  });

  it('two attestations for the same product get different IDs', async () => {
    const r1 = await makeAttestation({ summary: 'First' });
    const r2 = await makeAttestation({ summary: 'Second' });
    expect(r1.attestationId).not.toBe(r2.attestationId);
  });
});

// ── Retrieval ─────────────────────────────────────────────────────────────────

describe('getAttestation', () => {
  it('returns the record for a known ID', async () => {
    const record = await makeAttestation();
    const fetched = await getAttestation(record.attestationId);
    expect(fetched).not.toBeNull();
    expect(fetched!.attestationId).toBe(record.attestationId);
  });

  it('returns null for an unknown ID', async () => {
    const result = await getAttestation('att_nonexistent');
    expect(result).toBeNull();
  });
});

// ── Listing ───────────────────────────────────────────────────────────────────

describe('listAttestationsForProduct', () => {
  it('returns an empty array when no attestations exist', async () => {
    const result = await listAttestationsForProduct('unknown-product');
    expect(result).toEqual([]);
  });

  it('returns all attestations for a product', async () => {
    await makeAttestation({ productId: 'p1', summary: 'A' });
    await makeAttestation({ productId: 'p1', summary: 'B' });
    await makeAttestation({ productId: 'p2', summary: 'C' }); // different product

    const result = await listAttestationsForProduct('p1');
    expect(result).toHaveLength(2);
    expect(result.every((a) => a.productId === 'p1')).toBe(true);
  });

  it('includes a computed status field', async () => {
    await makeAttestation({ productId: 'p-status' });
    const result = await listAttestationsForProduct('p-status');
    expect(result[0].status).toBe('active');
  });

  it('returns results sorted newest first', async () => {
    const r1 = await makeAttestation({ productId: 'p-sort', summary: 'Older' });
    const r2 = await makeAttestation({ productId: 'p-sort', summary: 'Newer' });

    const result = await listAttestationsForProduct('p-sort');
    // Newer should come first (higher createdAt)
    expect(result[0].attestationId).toBe(r2.attestationId);
    expect(result[1].attestationId).toBe(r1.attestationId);
  });
});

describe('listAttestationsByIssuer', () => {
  it('returns all attestations by a given issuer', async () => {
    await makeAttestation({ issuerAddress: 'GISSUER_A', productId: 'p1' });
    await makeAttestation({ issuerAddress: 'GISSUER_A', productId: 'p2' });
    await makeAttestation({ issuerAddress: 'GISSUER_B', productId: 'p3' });

    const result = await listAttestationsByIssuer('GISSUER_A');
    expect(result).toHaveLength(2);
    expect(result.every((a) => a.issuerAddress === 'GISSUER_A')).toBe(true);
  });
});

// ── Revocation ────────────────────────────────────────────────────────────────

describe('revokeAttestation', () => {
  it('revokes an attestation when called by the issuer', async () => {
    const record = await makeAttestation({ issuerAddress: 'GISSUER' });
    const result = await revokeAttestation(record.attestationId, 'GISSUER', 'Audit superseded');

    expect(result.success).toBe(true);

    const updated = await getAttestation(record.attestationId);
    expect(updated!.revoked).toBe(true);
    expect(updated!.revokedAt).toBeGreaterThan(0);
    expect(updated!.revocationReason).toBe('Audit superseded');
  });

  it('returns an error when a non-issuer tries to revoke', async () => {
    const record = await makeAttestation({ issuerAddress: 'GISSUER' });
    const result = await revokeAttestation(record.attestationId, 'GOTHER');

    expect(result.success).toBe(false);
    expect(result.error).toMatch(/only the issuer/i);
  });

  it('returns an error for a non-existent attestation', async () => {
    const result = await revokeAttestation('att_nonexistent', 'GISSUER');
    expect(result.success).toBe(false);
    expect(result.error).toMatch(/not found/i);
  });

  it('is idempotent — revoking twice returns success', async () => {
    const record = await makeAttestation({ issuerAddress: 'GISSUER' });
    await revokeAttestation(record.attestationId, 'GISSUER');
    const result = await revokeAttestation(record.attestationId, 'GISSUER');
    expect(result.success).toBe(true);
  });

  it('preserves the record after revocation (audit trail)', async () => {
    const record = await makeAttestation({ issuerAddress: 'GISSUER', summary: 'Audit 2024' });
    await revokeAttestation(record.attestationId, 'GISSUER');

    const stored = await getAttestation(record.attestationId);
    expect(stored).not.toBeNull();
    expect(stored!.summary).toBe('Audit 2024');
  });

  it('shows status as revoked in list after revocation', async () => {
    const record = await makeAttestation({ productId: 'p-rev', issuerAddress: 'GISSUER' });
    await revokeAttestation(record.attestationId, 'GISSUER');

    const list = await listAttestationsForProduct('p-rev');
    expect(list[0].status).toBe('revoked');
  });
});

// ── Validation ────────────────────────────────────────────────────────────────

describe('validateAttestation', () => {
  it('returns valid=true for an active attestation', async () => {
    const record = await makeAttestation({ signedReference: 'ref-abc' });
    const result = await validateAttestation(record.attestationId);
    expect(result.valid).toBe(true);
    expect(result.record).toBeDefined();
  });

  it('returns valid=true when the reference matches', async () => {
    const record = await makeAttestation({ signedReference: 'ref-abc' });
    const result = await validateAttestation(record.attestationId, 'ref-abc');
    expect(result.valid).toBe(true);
  });

  it('returns valid=false when the reference does not match', async () => {
    const record = await makeAttestation({ signedReference: 'ref-abc' });
    const result = await validateAttestation(record.attestationId, 'ref-wrong');
    expect(result.valid).toBe(false);
    expect(result.reason).toMatch(/mismatch/i);
  });

  it('returns valid=false for a revoked attestation', async () => {
    const record = await makeAttestation({ issuerAddress: 'GISSUER' });
    await revokeAttestation(record.attestationId, 'GISSUER');

    const result = await validateAttestation(record.attestationId);
    expect(result.valid).toBe(false);
    expect(result.reason).toMatch(/revoked/i);
  });

  it('returns valid=false for an expired attestation', async () => {
    const record = await makeAttestation({ expiresInDays: 1 });

    // Manually expire the record
    const stored = await getAttestation(record.attestationId);
    stored!.expiresAt = Date.now() - 1000;
    const { kvStore } = await import('@/lib/kv');
    await kvStore.set(`attestation:${record.attestationId}`, JSON.stringify(stored!), 999);

    const result = await validateAttestation(record.attestationId);
    expect(result.valid).toBe(false);
    expect(result.reason).toMatch(/expired/i);
  });

  it('returns valid=false for a non-existent attestation', async () => {
    const result = await validateAttestation('att_nonexistent');
    expect(result.valid).toBe(false);
    expect(result.reason).toMatch(/not found/i);
  });
});
