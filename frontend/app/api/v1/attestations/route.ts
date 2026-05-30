/**
 * POST /api/v1/attestations  — Add an attestation to a product
 * GET  /api/v1/attestations  — List attestations (by productId or issuerAddress)
 *
 * Authentication: auditor tier or higher (x-api-key)
 * Rate limiting: default preset
 */

import { NextRequest, NextResponse } from 'next/server';
import { z } from 'zod';
import { withCors, handleOptions } from '@/lib/api/cors';
import { apiError, withCorrelationId, ErrorCode } from '@/lib/api/errors';
import { applyRateLimit, RATE_LIMIT_PRESETS } from '@/lib/api/rateLimit';
import { authenticateRegistryKey } from '@/lib/api/apiKeyAuth';
import { withIdempotency } from '@/lib/api/idempotency';
import { recordRequest } from '@/lib/api/metrics';
import {
  addAttestation,
  listAttestationsForProduct,
  listAttestationsByIssuer,
} from '@/lib/attestations';

export const runtime = 'nodejs';

// ── Validation ────────────────────────────────────────────────────────────────

const addAttestationSchema = z.object({
  productId: z
    .string()
    .trim()
    .min(1, 'productId is required')
    .max(128, 'productId must be 128 characters or fewer'),
  issuerAddress: z
    .string()
    .trim()
    .min(1, 'issuerAddress is required')
    .max(256, 'issuerAddress must be 256 characters or fewer'),
  issuerName: z
    .string()
    .trim()
    .min(1, 'issuerName is required')
    .max(256, 'issuerName must be 256 characters or fewer'),
  trustLevel: z.enum(['verified', 'trusted', 'community'] as const),
  attestationType: z.enum([
    'audit',
    'certification',
    'inspection',
    'compliance',
    'sustainability',
    'custom',
  ] as const),
  summary: z
    .string()
    .trim()
    .min(1, 'summary is required')
    .max(512, 'summary must be 512 characters or fewer'),
  signedReference: z
    .string()
    .trim()
    .min(1, 'signedReference is required')
    .max(2048, 'signedReference must be 2048 characters or fewer'),
  reportUrl: z.string().url('reportUrl must be a valid URL').optional(),
  expiresInDays: z.number().int().min(1).max(3650).optional(),
  metadata: z.string().max(4096, 'metadata must be 4096 characters or fewer').optional(),
});

// ── Handlers ──────────────────────────────────────────────────────────────────

export function OPTIONS(request: NextRequest) {
  return handleOptions(request);
}

export async function POST(request: NextRequest): Promise<NextResponse> {
  const start = Date.now();

  const limited = applyRateLimit(
    request,
    'POST /api/v1/attestations',
    RATE_LIMIT_PRESETS.default,
  );
  if (limited) {
    recordRequest('POST /api/v1/attestations', 429, Date.now() - start);
    return limited;
  }

  // Auditor tier or higher required
  const auth = await authenticateRegistryKey(
    request,
    'auditor',
    'POST /api/v1/attestations',
  );
  if (auth.error) {
    recordRequest('POST /api/v1/attestations', 401, Date.now() - start);
    return auth.error;
  }

  const response = await withIdempotency(request, async (req, rawBody) => {
    let payload: unknown;
    try {
      payload = JSON.parse(rawBody);
    } catch {
      return withCors(req, apiError(req, 400, ErrorCode.INVALID_JSON, 'Invalid JSON body'));
    }

    const parsed = addAttestationSchema.safeParse(payload);
    if (!parsed.success) {
      const details = parsed.error.issues.map((i) => ({
        field: i.path.join('.'),
        location: 'body' as const,
        message: i.message,
      }));
      return withCors(
        req,
        apiError(req, 400, ErrorCode.VALIDATION_ERROR, 'Validation failed', { details }),
      );
    }

    const record = await addAttestation(parsed.data);

    return withCors(
      req,
      withCorrelationId(req, NextResponse.json(record, { status: 201 })),
    );
  });

  recordRequest('POST /api/v1/attestations', response.status, Date.now() - start);
  return response;
}

export async function GET(request: NextRequest): Promise<NextResponse> {
  const start = Date.now();

  const limited = applyRateLimit(
    request,
    'GET /api/v1/attestations',
    RATE_LIMIT_PRESETS.publicRead,
  );
  if (limited) {
    recordRequest('GET /api/v1/attestations', 429, Date.now() - start);
    return limited;
  }

  const productId = request.nextUrl.searchParams.get('productId');
  const issuerAddress = request.nextUrl.searchParams.get('issuerAddress');

  if (!productId && !issuerAddress) {
    const res = withCors(
      request,
      apiError(
        request,
        400,
        ErrorCode.VALIDATION_ERROR,
        'Provide either productId or issuerAddress query parameter',
      ),
    );
    recordRequest('GET /api/v1/attestations', 400, Date.now() - start);
    return res;
  }

  const attestations = productId
    ? await listAttestationsForProduct(productId)
    : await listAttestationsByIssuer(issuerAddress!);

  const response = withCors(
    request,
    withCorrelationId(
      request,
      NextResponse.json({ attestations, total: attestations.length }, { status: 200 }),
    ),
  );

  recordRequest('GET /api/v1/attestations', response.status, Date.now() - start);
  return response;
}
