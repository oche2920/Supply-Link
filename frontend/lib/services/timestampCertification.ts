/**
 * Issue #503: Event timestamp certification service
 * Handles certification of event timestamps with cryptographic proof
 */

export interface EventTimestampCert {
  id: string;
  productId: string;
  eventStableId: string;
  certifiedTimestamp: number;
  certifier: string;
  issuedAt: number;
  revoked: boolean;
}

export interface TimestampCertificationRequest {
  productId: string;
  eventStableId: string;
  certifier: string;
}

/**
 * Certify an event's timestamp
 */
export async function certifyEventTimestamp(
  request: TimestampCertificationRequest,
): Promise<EventTimestampCert> {
  const response = await fetch('/api/certifications/timestamp', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    throw new Error(`Failed to certify timestamp: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Get all timestamp certifications for a product
 */
export async function getEventTimestampCerts(productId: string): Promise<EventTimestampCert[]> {
  const response = await fetch(`/api/certifications/timestamp?productId=${productId}`);

  if (!response.ok) {
    throw new Error(`Failed to fetch timestamp certifications: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Revoke a timestamp certification
 */
export async function revokeEventTimestampCert(
  productId: string,
  certId: string,
): Promise<boolean> {
  const response = await fetch(`/api/certifications/timestamp/${certId}`, {
    method: 'DELETE',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ productId }),
  });

  if (!response.ok) {
    throw new Error(`Failed to revoke timestamp certification: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Verify a timestamp certification
 */
export function verifyTimestampCert(cert: EventTimestampCert): boolean {
  return !cert.revoked && cert.certifiedTimestamp > 0;
}
