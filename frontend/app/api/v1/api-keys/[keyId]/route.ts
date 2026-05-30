/**
 * GET    /api/v1/api-keys/[keyId]  — Get key details + usage metrics
 * DELETE /api/v1/api-keys/[keyId]  — Revoke a key
 *
 * Authentication: internal tier API key required
 */

import { NextRequest, NextResponse } from 'next/server';
import { withCors, handleOptions } from '@/lib/api/cors';
import { apiError, withCorrelationId, ErrorCode } from '@/lib/api/errors';
import { applyRateLimit, RATE_LIMIT_PRESETS } from '@/lib/api/rateLimit';
import { authenticateApiRequest } from '@/lib/api/auth';
import { recordRequest } from '@/lib/api/metrics';
import { getApiKeyRecord, revokeApiKey, getApiKeyUsage } from '@/lib/api/apiKeyRegistry';

export const runtime = 'nodejs';

export function OPTIONS(request: NextRequest) {
  return handleOptions(request);
}

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ keyId: string }> },
): Promise<NextResponse> {
  const start = Date.now();
  const { keyId } = await params;

  const limited = applyRateLimit(
    request,
    'GET /api/v1/api-keys/[keyId]',
    RATE_LIMIT_PRESETS.default,
  );
  if (limited) {
    recordRequest('GET /api/v1/api-keys/[keyId]', 429, Date.now() - start);
    return limited;
  }

  const auth = await authenticateApiRequest(request, 'internal');
  if (auth.error) {
    recordRequest('GET /api/v1/api-keys/[keyId]', 401, Date.now() - start);
    return auth.error;
  }

  const record = await getApiKeyRecord(keyId);
  if (!record) {
    const res = withCors(request, apiError(request, 404, ErrorCode.VALIDATION_ERROR, 'API key not found'));
    recordRequest('GET /api/v1/api-keys/[keyId]', 404, Date.now() - start);
    return res;
  }

  const usage = await getApiKeyUsage(keyId);

  const body = {
    keyId: record.keyId,
    name: record.name,
    tier: record.tier,
    owner: record.owner,
    description: record.description,
    createdAt: record.createdAt,
    expiresAt: record.expiresAt,
    revoked: record.revoked,
    revokedAt: record.revokedAt,
    usage: usage
      ? {
          totalRequests: usage.totalRequests,
          windowRequests: usage.windowRequests,
          lastUsedAt: usage.lastUsedAt,
          endpointCounts: usage.endpointCounts,
        }
      : null,
  };

  const response = withCors(
    request,
    withCorrelationId(request, NextResponse.json(body, { status: 200 })),
  );
  recordRequest('GET /api/v1/api-keys/[keyId]', response.status, Date.now() - start);
  return response;
}

export async function DELETE(
  request: NextRequest,
  { params }: { params: Promise<{ keyId: string }> },
): Promise<NextResponse> {
  const start = Date.now();
  const { keyId } = await params;

  const limited = applyRateLimit(
    request,
    'DELETE /api/v1/api-keys/[keyId]',
    RATE_LIMIT_PRESETS.default,
  );
  if (limited) {
    recordRequest('DELETE /api/v1/api-keys/[keyId]', 429, Date.now() - start);
    return limited;
  }

  const auth = await authenticateApiRequest(request, 'internal');
  if (auth.error) {
    recordRequest('DELETE /api/v1/api-keys/[keyId]', 401, Date.now() - start);
    return auth.error;
  }

  const record = await getApiKeyRecord(keyId);
  if (!record) {
    const res = withCors(
      request,
      apiError(request, 404, ErrorCode.VALIDATION_ERROR, 'API key not found'),
    );
    recordRequest('DELETE /api/v1/api-keys/[keyId]', 404, Date.now() - start);
    return res;
  }

  await revokeApiKey(keyId);

  const response = withCors(
    request,
    withCorrelationId(
      request,
      NextResponse.json(
        { keyId, revoked: true, revokedAt: Date.now(), message: 'API key revoked successfully' },
        { status: 200 },
      ),
    ),
  );
  recordRequest('DELETE /api/v1/api-keys/[keyId]', response.status, Date.now() - start);
  return response;
}
