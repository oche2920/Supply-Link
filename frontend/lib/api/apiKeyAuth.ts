/**
 * Registry-based API key authentication middleware.
 *
 * Unlike the static env-var approach in auth.ts, this authenticates against
 * the dynamic API key registry (KV-backed), supports multiple keys per tier,
 * and tracks per-key usage counters.
 *
 * Usage:
 *   const auth = await authenticateRegistryKey(request, 'partner');
 *   if (auth.error) return auth.error;
 *   // auth.record is the ApiKeyRecord for the authenticated key
 */

import { NextRequest, NextResponse } from 'next/server';
import { apiError, ErrorCode } from '@/lib/api/errors';
import { authenticateKey, trackAndCheckUsage, type ApiKeyTier } from '@/lib/api/apiKeyRegistry';

export interface RegistryAuthResult {
  error: NextResponse | null;
  keyId?: string;
  tier?: ApiKeyTier;
}

/**
 * Authenticate a request using the API key registry.
 * Validates the key, checks revocation/expiry, and enforces per-key rate limits.
 *
 * @param request  The incoming NextRequest
 * @param tier     Required tier — requests with a lower-privilege key are rejected
 * @param endpoint Stable endpoint identifier used for usage tracking
 */
export async function authenticateRegistryKey(
  request: NextRequest,
  tier: ApiKeyTier,
  endpoint: string,
): Promise<RegistryAuthResult> {
  const rawKey = request.headers.get('x-api-key');

  if (!rawKey) {
    return {
      error: apiError(request, 401, ErrorCode.UNAUTHORIZED, 'Missing x-api-key header'),
    };
  }

  const record = await authenticateKey(rawKey);

  if (!record) {
    return {
      error: apiError(request, 401, ErrorCode.UNAUTHORIZED, 'Invalid or expired API key'),
    };
  }

  // Tier hierarchy: internal > partner > auditor
  // A higher-privilege key may access lower-privilege endpoints
  const tierRank: Record<ApiKeyTier, number> = { internal: 3, partner: 2, auditor: 1 };
  if (tierRank[record.tier] < tierRank[tier]) {
    return {
      error: apiError(
        request,
        403,
        ErrorCode.UNAUTHORIZED,
        `This endpoint requires '${tier}' tier or higher`,
      ),
    };
  }

  // Track usage and enforce per-key limits
  const usageResult = await trackAndCheckUsage(record.keyId, record.tier, endpoint);
  if (!usageResult.allowed) {
    const retryAfter = usageResult.retryAfterSeconds ?? 60;
    return {
      error: apiError(
        request,
        429,
        ErrorCode.RATE_LIMITED,
        'API key request limit exceeded for this window',
        { headers: { 'Retry-After': String(retryAfter) } },
      ),
    };
  }

  return { error: null, keyId: record.keyId, tier: record.tier };
}
