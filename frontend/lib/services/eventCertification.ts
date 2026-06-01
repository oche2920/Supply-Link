/**
 * Issue #505: Supply chain event certification workflows
 * Handles certifier registration and event certification
 */

export interface Certifier {
  id: string;
  address: string;
  name: string;
  certTypes: string[];
  registeredAt: number;
  active: boolean;
}

export interface EventCertification {
  id: string;
  productId: string;
  eventStableId: string;
  certType: string;
  certifier: string;
  metadata: string;
  issuedAt: number;
  revoked: boolean;
  revokedAt: number;
}

export interface CertifierRegistrationRequest {
  id: string;
  address: string;
  name: string;
  certTypes: string[];
}

export interface EventCertificationRequest {
  productId: string;
  eventStableId: string;
  certType: string;
  certifierId: string;
  metadata: string;
}

/**
 * Register a new certifier
 */
export async function registerCertifier(request: CertifierRegistrationRequest): Promise<Certifier> {
  const response = await fetch('/api/certifiers', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    throw new Error(`Failed to register certifier: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Get a certifier by ID
 */
export async function getCertifier(id: string): Promise<Certifier> {
  const response = await fetch(`/api/certifiers/${id}`);

  if (!response.ok) {
    throw new Error(`Failed to fetch certifier: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Certify a supply chain event
 */
export async function certifyEvent(
  request: EventCertificationRequest,
): Promise<EventCertification> {
  const response = await fetch('/api/event-certifications', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    throw new Error(`Failed to certify event: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Get all event certifications for a product
 */
export async function getEventCertifications(productId: string): Promise<EventCertification[]> {
  const response = await fetch(`/api/event-certifications?productId=${productId}`);

  if (!response.ok) {
    throw new Error(`Failed to fetch event certifications: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Revoke an event certification
 */
export async function revokeEventCertification(
  productId: string,
  certId: string,
): Promise<boolean> {
  const response = await fetch(`/api/event-certifications/${certId}`, {
    method: 'DELETE',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ productId }),
  });

  if (!response.ok) {
    throw new Error(`Failed to revoke event certification: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Verify an event certification
 */
export function verifyEventCertification(cert: EventCertification): boolean {
  return !cert.revoked && cert.issuedAt > 0;
}

/**
 * Get certification types for a certifier
 */
export function getCertificationTypes(certifier: Certifier): string[] {
  return certifier.certTypes;
}

/**
 * Check if certifier is authorized for a cert type
 */
export function isCertifierAuthorized(certifier: Certifier, certType: string): boolean {
  return certifier.active && certifier.certTypes.includes(certType);
}
