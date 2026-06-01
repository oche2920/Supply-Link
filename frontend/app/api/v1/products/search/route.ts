/**
 * POST /api/v1/products/search – search and filter products
 *
 * Request body:
 * {
 *   "text": "coffee",
 *   "filters": {
 *     "category": "agricultural",
 *     "status": "active"
 *   },
 *   "offset": 0,
 *   "limit": 50
 * }
 */

import { NextRequest, NextResponse } from 'next/server';
import { withCors, handleOptions } from '@/lib/api/cors';
import { apiError, withCorrelationId, ErrorCode } from '@/lib/api/errors';
import { applyRateLimit, RATE_LIMIT_PRESETS } from '@/lib/api/rateLimit';
import { authenticateApiRequest } from '@/lib/api/auth';
import { searchProducts } from '@/lib/services/searchService';
import { getAllProducts } from '@/lib/mock/products';
import { recordRequest } from '@/lib/api/metrics';

export function OPTIONS(request: NextRequest) {
  return handleOptions(request);
}

export async function POST(request: NextRequest): Promise<NextResponse> {
  const start = Date.now();

  const limited = applyRateLimit(
    request,
    'POST /api/v1/products/search',
    RATE_LIMIT_PRESETS.default,
  );
  if (limited) {
    recordRequest('POST /api/v1/products/search', 429, Date.now() - start);
    return limited;
  }

  const auth = await authenticateApiRequest(request, 'partner');
  if (auth.error) {
    recordRequest('POST /api/v1/products/search', 401, Date.now() - start);
    return auth.error;
  }

  let payload: unknown;
  try {
    payload = JSON.parse(await request.text());
  } catch {
    return apiError(request, 400, ErrorCode.INVALID_PAYLOAD, 'Invalid JSON');
  }

  const body = payload as Record<string, unknown>;
  const query = {
    text: typeof body.text === 'string' ? body.text : undefined,
    filters: body.filters as Record<string, unknown> | undefined,
    offset: typeof body.offset === 'number' ? body.offset : 0,
    limit: typeof body.limit === 'number' ? body.limit : 50,
  };

  const products = getAllProducts();
  const result = searchProducts(products, query);

  recordRequest('POST /api/v1/products/search', 200, Date.now() - start);
  return withCors(request, withCorrelationId(request, NextResponse.json(result, { status: 200 })));
}
