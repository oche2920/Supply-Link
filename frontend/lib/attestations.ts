/**
 * Attestation Registry
 *
 * Third-party auditors can attach signed attestations to products.
 * Attestations carry:
 *   - Issuer identity (wallet address or service identifier)
 *   - Trust level (verified | trusted | community)
 *   - Attestation type (audit | certification | inspection | compliance | custom)
 *   - A signed reference (hash or URL of the off-chain audit document)
 *   - Revocation support
 *
 * Storage: Vercel KV in production, in-memory Map in dev/test.
 *
 * KV layout:
 *   attestation:<attestationId>          → AttestationRecord (JSON)
 *   attestation:list:<productId>         → string[] of attestationIds for a product
 *   attestation:issuer:<issuerAddress>   → string[] of attestationIds by issuer
 */

import { randomBytes } from 'crypto';
import { kvStore } from '@/lib/kv';

// ── Types ─────────────────────────────────────────────────────────────────────

export type AttestationTrustLevel = 'verified' | 'trusted' | 'community';

export type AttestationType =
  | 'audit'
  | 'certification'
  | 'inspection'
  | 'compliance'
  | 'sustainability'
  | 'custom';

export interface AttestationRecord {
  /** Stable unique identifier */
  attestationId: string;
  /** Product this attestation is attached to */
  productId: string;
  /** Stellar wallet address or service identifier of the issuer */
  issuerAddress: string;
  /** Human-readable name of the issuing organisation */
  issuerName: string;
  /** Trust level assigned to this attestation */
  trustLevel: AttestationTrustLevel;
  /** Category of attestation */
  attestationType: AttestationType;
  /** Short human-readable summary */
  summary: string;
  /**
   * Signed reference — a hex-encoded SHA-256 hash of the off-chain audit
   * document, or a URL to the document. Stored opaquely; consumers validate
   * the hash against the document themselves.
   */
  signedReference: string;
  /** Optional URL to the full audit report */
  reportUrl?: string;
  /** Unix ms timestamp when the attestation was created */
  createdAt: number;
  /** Unix ms expiry; 0 = never expires */
  expiresAt: number;
  /** Whether this attestation has been revoked */
  revoked: boolean;
  /** Unix ms timestamp when revoked; 0 if not revoked */
  revokedAt: number;
  /** Reason for revocation */
  revocationReason?: string;
  /** Optional metadata JSON string */
  metadata?: string;
}

/** Public view — omits internal fields, adds computed status */
export interface AttestationView extends AttestationRecord {
  /** Computed: 'active' | 'revoked' | 'expired' */
  status: 'active' | 'revoked' | 'expired';
}

// ── KV key helpers ────────────────────────────────────────────────────────────

function attestationKey(attestationId: string): string {
  return `attestation:${attestationId}`;
}

function productListKey(productId: string): string {
  return `attestation:list:${productId}`;
}

function issuerListKey(issuerAddress: string): string {
  return `attestation:issuer:${issuerAddress}`;
}

const RECORD_TTL = 365 * 24 * 60 * 60; // 1 year
const LIST_TTL = 365 * 24 * 60 * 60;

// ── Helpers ───────────────────────────────────────────────────────────────────

function computeStatus(record: AttestationRecord): 'active' | 'revoked' | 'expired' {
  if (record.revoked) return 'revoked';
  if (record.expiresAt > 0 && Date.now() > record.expiresAt) return 'expired';
  return 'active';
}

function toView(record: AttestationRecord): AttestationView {
  return { ...record, status: computeStatus(record) };
}

async function appendToList(listKey: string, id: string, ttl: number): Promise<void> {
  const raw = await kvStore.get(listKey);
  const list: string[] = raw ? JSON.parse(raw) : [];
  if (!list.includes(id)) {
    list.push(id);
    await kvStore.set(listKey, JSON.stringify(list), ttl);
  }
}

// ── CRUD ──────────────────────────────────────────────────────────────────────

/**
 * Add a new attestation to a product.
 */
export async function addAttestation(params: {
  productId: string;
  issuerAddress: string;
  issuerName: string;
  trustLevel: AttestationTrustLevel;
  attestationType: AttestationType;
  summary: string;
  signedReference: string;
  reportUrl?: string;
  expiresInDays?: number;
  metadata?: string;
}): Promise<AttestationRecord> {
  const attestationId = `att_${randomBytes(12).toString('hex')}`;
  const now = Date.now();
  const expiresAt =
    params.expiresInDays && params.expiresInDays > 0
      ? now + params.expiresInDays * 24 * 60 * 60 * 1000
      : 0;

  const record: AttestationRecord = {
    attestationId,
    productId: params.productId,
    issuerAddress: params.issuerAddress,
    issuerName: params.issuerName,
    trustLevel: params.trustLevel,
    attestationType: params.attestationType,
    summary: params.summary,
    signedReference: params.signedReference,
    reportUrl: params.reportUrl,
    createdAt: now,
    expiresAt,
    revoked: false,
    revokedAt: 0,
    metadata: params.metadata,
  };

  await kvStore.set(attestationKey(attestationId), JSON.stringify(record), RECORD_TTL);
  await appendToList(productListKey(params.productId), attestationId, LIST_TTL);
  await appendToList(issuerListKey(params.issuerAddress), attestationId, LIST_TTL);

  return record;
}

/**
 * Retrieve a single attestation by ID.
 */
export async function getAttestation(attestationId: string): Promise<AttestationRecord | null> {
  const raw = await kvStore.get(attestationKey(attestationId));
  if (!raw) return null;
  return JSON.parse(raw) as AttestationRecord;
}

/**
 * List all attestations for a product (active, revoked, and expired).
 */
export async function listAttestationsForProduct(productId: string): Promise<AttestationView[]> {
  const raw = await kvStore.get(productListKey(productId));
  if (!raw) return [];

  const ids: string[] = JSON.parse(raw);
  const records = await Promise.all(ids.map((id) => getAttestation(id)));
  return records
    .filter((r): r is AttestationRecord => r !== null)
    .map(toView)
    .sort((a, b) => b.createdAt - a.createdAt);
}

/**
 * List all attestations issued by a specific address.
 */
export async function listAttestationsByIssuer(issuerAddress: string): Promise<AttestationView[]> {
  const raw = await kvStore.get(issuerListKey(issuerAddress));
  if (!raw) return [];

  const ids: string[] = JSON.parse(raw);
  const records = await Promise.all(ids.map((id) => getAttestation(id)));
  return records
    .filter((r): r is AttestationRecord => r !== null)
    .map(toView)
    .sort((a, b) => b.createdAt - a.createdAt);
}

/**
 * Revoke an attestation. Only the original issuer may revoke.
 * Returns false if the attestation is not found or the caller is not the issuer.
 */
export async function revokeAttestation(
  attestationId: string,
  callerAddress: string,
  reason?: string,
): Promise<{ success: boolean; error?: string }> {
  const record = await getAttestation(attestationId);
  if (!record) return { success: false, error: 'Attestation not found' };
  if (record.issuerAddress !== callerAddress) {
    return { success: false, error: 'Only the issuer may revoke this attestation' };
  }
  if (record.revoked) return { success: true }; // idempotent

  record.revoked = true;
  record.revokedAt = Date.now();
  record.revocationReason = reason;

  await kvStore.set(attestationKey(attestationId), JSON.stringify(record), RECORD_TTL);
  return { success: true };
}

/**
 * Validate an attestation: check it exists, is active, and the signed reference
 * matches the provided hash.
 */
export async function validateAttestation(
  attestationId: string,
  expectedReference?: string,
): Promise<{ valid: boolean; reason?: string; record?: AttestationView }> {
  const record = await getAttestation(attestationId);
  if (!record) return { valid: false, reason: 'Attestation not found' };

  const view = toView(record);

  if (view.status === 'revoked') return { valid: false, reason: 'Attestation has been revoked', record: view };
  if (view.status === 'expired') return { valid: false, reason: 'Attestation has expired', record: view };

  if (expectedReference && record.signedReference !== expectedReference) {
    return { valid: false, reason: 'Signed reference mismatch', record: view };
  }

  return { valid: true, record: view };
}
