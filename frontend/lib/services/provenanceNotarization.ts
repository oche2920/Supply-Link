/**
 * Issue #504: Product provenance proof notarization service
 * Handles notarization of product provenance proofs
 */

export interface ProvenanceNotarization {
  id: string;
  productId: string;
  proofHash: string;
  notary: string;
  notarizedAt: number;
  expiresAt: number;
  revoked: boolean;
}

export interface NotarizationRequest {
  productId: string;
  proofHash: string;
  notary: string;
  expiresAt: number;
}

/**
 * Notarize a product's provenance proof
 */
export async function notarizeProvenance(
  request: NotarizationRequest,
): Promise<ProvenanceNotarization> {
  const response = await fetch('/api/notarizations/provenance', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    throw new Error(`Failed to notarize provenance: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Get all provenance notarizations for a product
 */
export async function getProvenanceNotarizations(
  productId: string,
): Promise<ProvenanceNotarization[]> {
  const response = await fetch(`/api/notarizations/provenance?productId=${productId}`);

  if (!response.ok) {
    throw new Error(`Failed to fetch provenance notarizations: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Revoke a provenance notarization
 */
export async function revokeProvenanceNotarization(
  productId: string,
  notarizationId: string,
): Promise<boolean> {
  const response = await fetch(`/api/notarizations/provenance/${notarizationId}`, {
    method: 'DELETE',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ productId }),
  });

  if (!response.ok) {
    throw new Error(`Failed to revoke provenance notarization: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Verify a provenance notarization
 */
export function verifyProvenanceNotarization(notarization: ProvenanceNotarization): boolean {
  const now = Math.floor(Date.now() / 1000);
  return !notarization.revoked && (notarization.expiresAt === 0 || notarization.expiresAt > now);
}

/**
 * Calculate proof hash from product history
 */
export async function calculateProofHash(productId: string): Promise<string> {
  const response = await fetch(`/api/notarizations/calculate-hash?productId=${productId}`);

  if (!response.ok) {
    throw new Error(`Failed to calculate proof hash: ${response.statusText}`);
  }

  const data = await response.json();
  return data.hash;
}
