/**
 * API Key Registry
 *
 * Manages the full lifecycle of API keys for external machine-to-machine consumers:
 *   - Issuance: generate a cryptographically random key with metadata
 *   - Revocation: mark a key as revoked (soft-delete, preserved for audit)
 *   - Usage tracking: per-key request counters and last-used timestamps
 *   - Limit enforcement: configurable per-key request limits
 *
 * Storage: Vercel KV in production, in-memory Map in dev/test.
 *
 * Key format: `sl_<tier>_<32-byte hex>` — e.g. `sl_partner_abc123...`
 * KV layout:
 *   apikey:<keyId>          → ApiKeyRecord (JSON)
 *   apikey:list             → string[] of all keyIds (for management listing)
 *   apikey:usage:<keyId>    → ApiKeyUsage (JSON)
 */

import { randomBytes } from 'crypto';
import { kvStore } from '@/lib/kv';

// ── Types ─────────────────────────────────────────────────────────────────────

export type ApiKeyTier = 'partner' | 'internal' | 'auditor';

export interface ApiKeyRecord {
  /** Stable unique identifier (not the secret key itself) */
  keyId: string;
  /** The actual secret key — only returned once at issuance */
  keyHash: string;
  /** Human-readable name for this key */
  name: string;
  /** Access tier controlling rate limits */
  tier: ApiKeyTier;
  /** Unix ms timestamp when the key was created */
  createdAt: number;
  /** Unix ms expiry timestamp; 0 = never expires */
  expiresAt: number;
  /** Whether the key has been revoked */
  revoked: boolean;
  /** Unix ms timestamp when the key was revoked; 0 if not revoked */
  revokedAt: number;
  /** Optional description / notes */
  description?: string;
  /** Owner identifier (wallet address or service name) */
  owner: string;
}

export interface ApiKeyUsage {
  keyId: string;
  /** Total requests made with this key */
  totalRequests: number;
  /** Requests in the current rolling window */
  windowRequests: number;
  /** Unix ms timestamp of the last request */
  lastUsedAt: number;
  /** Unix ms timestamp when the current window started */
  windowStartedAt: number;
  /** Per-endpoint breakdown: endpoint → count */
  endpointCounts: Record<string, number>;
}

export interface ApiKeyLimits {
  /** Max requests per window */
  requestsPerWindow: number;
  /** Window duration in ms */
  windowMs: number;
}

/** Limits per tier */
export const API_KEY_TIER_LIMITS: Record<ApiKeyTier, ApiKeyLimits> = {
  partner: { requestsPerWindow: 1_000, windowMs: 60_000 },
  internal: { requestsPerWindow: 5_000, windowMs: 60_000 },
  auditor: { requestsPerWindow: 500, windowMs: 60_000 },
};

/** TTL for KV records (90 days) */
const RECORD_TTL_SECONDS = 90 * 24 * 60 * 60;
/** TTL for usage records (7 days rolling) */
const USAGE_TTL_SECONDS = 7 * 24 * 60 * 60;
/** TTL for the key list index (90 days) */
const LIST_TTL_SECONDS = 90 * 24 * 60 * 60;

// ── Key generation ────────────────────────────────────────────────────────────

/**
 * Generate a new API key secret.
 * Returns both the plaintext key (shown once) and its SHA-256 hash (stored).
 */
export async function generateApiKey(tier: ApiKeyTier): Promise<{
  plaintext: string;
  hash: string;
  keyId: string;
}> {
  const secret = randomBytes(32).toString('hex');
  const keyId = `kid_${randomBytes(8).toString('hex')}`;
  const plaintext = `sl_${tier}_${secret}`;

  // Hash the key for storage — never store plaintext
  const encoder = new TextEncoder();
  const data = encoder.encode(plaintext);
  const hashBuffer = await crypto.subtle.digest('SHA-256', data);
  const hash = Array.from(new Uint8Array(hashBuffer))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');

  return { plaintext, hash, keyId };
}

/**
 * Hash an incoming key for comparison against stored hashes.
 */
export async function hashApiKey(plaintext: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(plaintext);
  const hashBuffer = await crypto.subtle.digest('SHA-256', data);
  return Array.from(new Uint8Array(hashBuffer))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

// ── Storage helpers ───────────────────────────────────────────────────────────

function recordKey(keyId: string): string {
  return `apikey:${keyId}`;
}

function usageKey(keyId: string): string {
  return `apikey:usage:${keyId}`;
}

const LIST_KEY = 'apikey:list';

// ── CRUD operations ───────────────────────────────────────────────────────────

/**
 * Issue a new API key.
 * Returns the record (without plaintext) and the plaintext key (shown once).
 */
export async function issueApiKey(params: {
  name: string;
  tier: ApiKeyTier;
  owner: string;
  description?: string;
  expiresInDays?: number;
}): Promise<{ record: ApiKeyRecord; plaintext: string }> {
  const { plaintext, hash, keyId } = await generateApiKey(params.tier);

  const now = Date.now();
  const expiresAt =
    params.expiresInDays && params.expiresInDays > 0
      ? now + params.expiresInDays * 24 * 60 * 60 * 1000
      : 0;

  const record: ApiKeyRecord = {
    keyId,
    keyHash: hash,
    name: params.name,
    tier: params.tier,
    createdAt: now,
    expiresAt,
    revoked: false,
    revokedAt: 0,
    description: params.description,
    owner: params.owner,
  };

  await kvStore.set(recordKey(keyId), JSON.stringify(record), RECORD_TTL_SECONDS);

  // Append to the global list index
  const rawList = await kvStore.get(LIST_KEY);
  const list: string[] = rawList ? JSON.parse(rawList) : [];
  list.push(keyId);
  await kvStore.set(LIST_KEY, JSON.stringify(list), LIST_TTL_SECONDS);

  // Initialise usage record
  const usage: ApiKeyUsage = {
    keyId,
    totalRequests: 0,
    windowRequests: 0,
    lastUsedAt: 0,
    windowStartedAt: now,
    endpointCounts: {},
  };
  await kvStore.set(usageKey(keyId), JSON.stringify(usage), USAGE_TTL_SECONDS);

  return { record, plaintext };
}

/**
 * Retrieve a key record by keyId.
 */
export async function getApiKeyRecord(keyId: string): Promise<ApiKeyRecord | null> {
  const raw = await kvStore.get(recordKey(keyId));
  if (!raw) return null;
  return JSON.parse(raw) as ApiKeyRecord;
}

/**
 * Revoke an API key. Soft-delete: the record is preserved for audit.
 */
export async function revokeApiKey(keyId: string): Promise<boolean> {
  const record = await getApiKeyRecord(keyId);
  if (!record) return false;
  if (record.revoked) return true; // idempotent

  record.revoked = true;
  record.revokedAt = Date.now();

  await kvStore.set(recordKey(keyId), JSON.stringify(record), RECORD_TTL_SECONDS);
  return true;
}

/**
 * List all API key records (active and revoked).
 */
export async function listApiKeys(): Promise<ApiKeyRecord[]> {
  const rawList = await kvStore.get(LIST_KEY);
  if (!rawList) return [];

  const keyIds: string[] = JSON.parse(rawList);
  const records = await Promise.all(keyIds.map((id) => getApiKeyRecord(id)));
  return records.filter((r): r is ApiKeyRecord => r !== null);
}

// ── Authentication ────────────────────────────────────────────────────────────

/**
 * Authenticate an incoming API key.
 * Returns the matching record if valid and not revoked/expired, null otherwise.
 */
export async function authenticateKey(plaintext: string): Promise<ApiKeyRecord | null> {
  if (!plaintext || !plaintext.startsWith('sl_')) return null;

  const hash = await hashApiKey(plaintext);

  // Scan all keys to find a hash match
  // In production this would use a secondary index; for KV this is acceptable
  // given the expected number of keys (tens, not millions).
  const records = await listApiKeys();
  const record = records.find((r) => r.keyHash === hash);

  if (!record) return null;
  if (record.revoked) return null;
  if (record.expiresAt > 0 && Date.now() > record.expiresAt) return null;

  return record;
}

// ── Usage tracking ────────────────────────────────────────────────────────────

/**
 * Record a request against a key and check if the limit is exceeded.
 * Returns { allowed, retryAfterSeconds } — callers must honour the result.
 */
export async function trackAndCheckUsage(
  keyId: string,
  tier: ApiKeyTier,
  endpoint: string,
): Promise<{ allowed: boolean; retryAfterSeconds?: number }> {
  const limits = API_KEY_TIER_LIMITS[tier];
  const now = Date.now();

  const raw = await kvStore.get(usageKey(keyId));
  let usage: ApiKeyUsage = raw
    ? (JSON.parse(raw) as ApiKeyUsage)
    : {
        keyId,
        totalRequests: 0,
        windowRequests: 0,
        lastUsedAt: 0,
        windowStartedAt: now,
        endpointCounts: {},
      };

  // Reset window if expired
  if (now - usage.windowStartedAt >= limits.windowMs) {
    usage.windowRequests = 0;
    usage.windowStartedAt = now;
  }

  // Check limit
  if (usage.windowRequests >= limits.requestsPerWindow) {
    const windowEndsAt = usage.windowStartedAt + limits.windowMs;
    const retryAfterSeconds = Math.max(1, Math.ceil((windowEndsAt - now) / 1000));
    return { allowed: false, retryAfterSeconds };
  }

  // Increment counters
  usage.totalRequests++;
  usage.windowRequests++;
  usage.lastUsedAt = now;
  usage.endpointCounts[endpoint] = (usage.endpointCounts[endpoint] ?? 0) + 1;

  await kvStore.set(usageKey(keyId), JSON.stringify(usage), USAGE_TTL_SECONDS);

  return { allowed: true };
}

/**
 * Retrieve usage metrics for a key.
 */
export async function getApiKeyUsage(keyId: string): Promise<ApiKeyUsage | null> {
  const raw = await kvStore.get(usageKey(keyId));
  if (!raw) return null;
  return JSON.parse(raw) as ApiKeyUsage;
}
