/**
 * GET    /api/v1/attestations/[attestationId]          — Get attestation details
 * DELETE /api/v1/attestations/[attestationId]          — Revoke an attestation
 * GET    /api/v1/attestations/[attestationId]/validate — Validate an attestation
 */

import { NextRequest, NextResponse } from 'next/server';
import { withCors, handleOptions } from '@/lib/api/cors';
import { apiError, withCorrelationId, ErrorCode } from '@/lib/api/errors';
import { applyRateLimit, RATE_LIMIT_PRESETS } from '@/lib/api/rateLimit';
import { authenticateRegistryKey } from '@/lib/api/apiKeyAuth';
import { recordRequest } from '@/lib/api/metrics';
import { getAttestation, revokeAttestation } from '@/lib/attestations';

export const runtime = 'nodejs';

export function OPTIONS(request: NextRequest) {
  return handleOptions(request);
}

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ attestationId: string }> },
): Promise<NextResponse> {
  const start = Date.now();
  const { attestationId } = await params;

  const limited = applyRateLimit(
    request,
    'GET /api/v1/attestations/[id]',
    RATE_LIMIT_PRESETS.publicRead,
  );
  if (limited) {
    recordRequest('GET /api/v1/attestations/[id]', 429, Date.now() - start);
    return limited;
  }

  const record = await getAttestation(attestationId);
  if (!record) {
    const res = withCors(
      request,
      apiError(request, 404, ErrorCode.VALIDATION_ERROR, 'Attestation not found'),
    );
    recordRequest('GET /api/v1/attestations/[id]', 404, Date.now() - start);
    return res;
  }

  const response = withCors(
    request,
    withCorrelationId(request, NextResponse.json(record, { status: 200 })),
  );
  recordRequest('GET /api/v1/attestations/[id]', response.status, Date.now() - start);
  return response;
}

export async function DELETE(
  request: NextRequest,
  { params }: { params: Promise<{ attestationId: string }> },
): Promise<NextResponse> {
  const start = Date.now();
  const { attestationId } = await params;

  const limited = applyRateLimit(
    request,
    'DELETE /api/v1/attestations/[id]',
    RATE_LIMIT_PRESETS.default,
  );
  if (limited) {
    recordRequest('DELETE /api/v1/attestations/[id]', 429, Date.now() - start);
    return limited;
  }

  // Auditor tier or higher required to revoke
  const auth = await authenticateRegistryKey(
    request,
    'auditor',
    'DELETE /api/v1/attestations/[id]',
  );
  if (auth.error) {
    recordRequest('DELETE /api/v1/attestations/[id]', 401, Date.now() - start);
    return auth.error;
  }

  // Caller must identify themselves via x-issuer-address header
  const callerAddress = request.headers.get('x-issuer-address');
  if (!callerAddress) {
    const res = withCors(
      request,
      apiError(request, 400, ErrorCode.MISSING_FIELDS, 'Missing x-issuer-address header'),
    );
    recordRequest('DELETE /api/v1/attestations/[id]', 400, Date.now() - start);
    return res;
  }

  let reason: string | undefined;
  try {
    const body = await request.json().catch(() => ({}));
    reason = typeof body?.reason === 'string' ? body.reason : undefined;
  } catch {
    // body is optional
  }

  const result = await revokeAttestation(attestationId, callerAddress, reason);

  if (!result.success) {
    const status = result.error === 'Attestation not found' ? 404 : 403;
    const res = withCors(
      request,
      apiError(request, status, ErrorCode.UNAUTHORIZED, result.error ?? 'Revocation failed'),
    );
    recordRequest('DELETE /api/v1/attestations/[id]', status, Date.now() - start);
    return res;
  }

  const response = withCors(
    request,
    withCorrelationId(
      request,
      NextResponse.json(
        { attestationId, revoked: true, revokedAt: Date.now() },
        { status: 200 },
      ),
    ),
  );
  recordRequest('DELETE /api/v1/attestations/[id]', response.status, Date.now() - start);
  return response;
}
