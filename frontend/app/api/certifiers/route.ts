import { NextRequest, NextResponse } from 'next/server';

/**
 * Issue #505: Certifier management API
 * POST /api/certifiers - Register a new certifier
 * GET /api/certifiers/[id] - Get a certifier by ID
 */

export async function POST(request: NextRequest) {
  try {
    const body = await request.json();
    const { id, address, name, certTypes } = body;

    if (!id || !address || !name || !certTypes) {
      return NextResponse.json({ error: 'Missing required fields' }, { status: 400 });
    }

    // Mock implementation - in production, this would call the smart contract
    const certifier = {
      id,
      address,
      name,
      certTypes,
      registeredAt: Math.floor(Date.now() / 1000),
      active: true,
    };

    return NextResponse.json(certifier);
  } catch (error) {
    return NextResponse.json({ error: 'Failed to register certifier' }, { status: 500 });
  }
}

export async function GET(request: NextRequest) {
  try {
    const id = request.nextUrl.searchParams.get('id');

    if (!id) {
      return NextResponse.json({ error: 'Missing id parameter' }, { status: 400 });
    }

    // Mock implementation - return a sample certifier
    const certifier = {
      id,
      address: 'GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX',
      name: 'Sample Certifier',
      certTypes: ['quality_check', 'compliance_verified'],
      registeredAt: Math.floor(Date.now() / 1000),
      active: true,
    };

    return NextResponse.json(certifier);
  } catch (error) {
    return NextResponse.json({ error: 'Failed to fetch certifier' }, { status: 500 });
  }
}
