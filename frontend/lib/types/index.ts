export type EventType = "HARVEST" | "PROCESSING" | "SHIPPING" | "RETAIL";

export interface OwnershipRecord {
  owner: string; // Stellar address
  transferredAt: number; // unix ms
}

export interface Product {
  id: string;
  name: string;
  origin: string;
  owner: string; // Stellar address
  timestamp: number;
  active: boolean;
  authorizedActors: string[];
  ownershipHistory?: OwnershipRecord[];
}

export interface TrackingEvent {
  productId: string;
  location: string;
  actor: string; // Stellar address
  timestamp: number;
  eventType: EventType;
  metadata: string; // JSON string
  archived?: boolean;
  archivedAt?: number; // unix ms, 0 if not archived
}

export type CertStatus = "ACTIVE" | "REVOKED";

export interface Certification {
  certId: string;
  productId: string;
  issuer: string; // Stellar address
  issuedAt: number; // unix ms
  certType: string; // e.g. "ORGANIC", "FAIR_TRADE", "ISO9001"
  reference: string; // external registry URL / ID
  revoked: boolean;
  revokedAt?: number; // unix ms
}

// ── Risk scoring ──────────────────────────────────────────────────────────────

export type RiskLevel = "LOW" | "MEDIUM" | "HIGH" | "CRITICAL";

export interface RiskFactor {
  id: string;
  label: string;
  description: string;
  score: number; // points contributed (0–100 scale)
}

export interface RiskScore {
  total: number;       // 0–100
  level: RiskLevel;
  factors: RiskFactor[];
}
