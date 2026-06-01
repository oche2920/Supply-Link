import { NextRequest, NextResponse } from 'next/server';

/**
 * Get anomaly summary for a product
 */
export async function GET(request: NextRequest) {
  try {
    const productId = request.nextUrl.searchParams.get('productId');

    if (!productId) {
      return NextResponse.json({ error: 'Missing productId parameter' }, { status: 400 });
    }

    // Mock implementation - return sample summary
    const summary = {
      totalAnomalies: 3,
      criticalCount: 0,
      highCount: 1,
      mediumCount: 2,
      lowCount: 0,
      reviewedCount: 1,
      pendingCount: 2,
    };

    return NextResponse.json(summary);
  } catch (error) {
    return NextResponse.json({ error: 'Failed to fetch anomaly summary' }, { status: 500 });
  }
}
