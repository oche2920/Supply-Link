import { NextRequest, NextResponse } from 'next/server';
import crypto from 'crypto';

/**
 * Calculate proof hash for a product's provenance
 */
export async function POST(request: NextRequest) {
  try {
    const body = await request.json();
    const { productId } = body;

    if (!productId) {
      return NextResponse.json({ error: 'Missing productId' }, { status: 400 });
    }

    // Mock implementation - generate a deterministic hash based on productId
    const hash = crypto.createHash('sha256').update(productId).digest('hex');

    return NextResponse.json({ hash });
  } catch (error) {
    return NextResponse.json({ error: 'Failed to calculate proof hash' }, { status: 500 });
  }
}

export async function GET(request: NextRequest) {
  try {
    const productId = request.nextUrl.searchParams.get('productId');

    if (!productId) {
      return NextResponse.json({ error: 'Missing productId parameter' }, { status: 400 });
    }

    // Mock implementation - generate a deterministic hash
    const hash = crypto.createHash('sha256').update(productId).digest('hex');

    return NextResponse.json({ hash });
  } catch (error) {
    return NextResponse.json({ error: 'Failed to calculate proof hash' }, { status: 500 });
  }
}
