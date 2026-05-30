/**
 * Recall Risk Scoring Engine
 *
 * Assigns a 0–100 risk score to a product based on its event history and
 * certification state. Higher scores indicate greater recall risk.
 *
 * Score bands:
 *   0–24   LOW      — normal, no anomalies detected
 *   25–49  MEDIUM   — minor irregularities, worth monitoring
 *   50–74  HIGH     — significant anomalies, review recommended
 *   75–100 CRITICAL — severe irregularities, immediate attention required
 *
 * Risk factors (each contributes a fixed number of points):
 *
 *   F1  NO_EVENTS            +30  Product has no tracking events at all
 *   F2  MISSING_HARVEST      +20  No HARVEST event in the lifecycle
 *   F3  WRONG_ORDER          +25  Events appear out of expected stage order
 *   F4  LOCATION_JUMP        +20  Consecutive events jump >10,000 km (heuristic)
 *   F5  LONG_GAP             +15  Gap of >30 days between consecutive events
 *   F6  UNAUTHORIZED_ACTOR   +25  An event actor is not in authorizedActors
 *   F7  HIGH_ARCHIVE_RATIO   +15  >50% of total events have been archived
 *   F8  NO_CERTIFICATIONS    +10  Product has zero certifications
 *   F9  REVOKED_CERT         +15  At least one certification has been revoked
 *   F10 STALE_METADATA       +10  Any event carries empty or trivial metadata
 *
 * Total is capped at 100.
 */

import type { Product, TrackingEvent, Certification, RiskScore, RiskFactor, RiskLevel } from "@/lib/types";

// Expected lifecycle order — earlier stages should precede later ones
const STAGE_ORDER: Record<string, number> = {
  HARVEST: 0,
  PROCESSING: 1,
  SHIPPING: 2,
  RETAIL: 3,
};

// Rough haversine — good enough for anomaly detection, not navigation
function haversineKm(
  lat1: number, lon1: number,
  lat2: number, lon2: number
): number {
  const R = 6371;
  const dLat = ((lat2 - lat1) * Math.PI) / 180;
  const dLon = ((lon2 - lon1) * Math.PI) / 180;
  const a =
    Math.sin(dLat / 2) ** 2 +
    Math.cos((lat1 * Math.PI) / 180) *
      Math.cos((lat2 * Math.PI) / 180) *
      Math.sin(dLon / 2) ** 2;
  return R * 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a));
}

// Very naive geocoder — maps known city substrings to approximate coords.
// In production this would call a real geocoding API.
const CITY_COORDS: Array<[string, number, number]> = [
  ["ethiopia", 9.0, 38.7],
  ["addis", 9.0, 38.7],
  ["djibouti", 11.6, 43.1],
  ["rotterdam", 51.9, 4.5],
  ["amsterdam", 52.4, 4.9],
  ["shanghai", 31.2, 121.5],
  ["hamburg", 53.6, 10.0],
  ["singapore", 1.3, 103.8],
  ["ghana", 7.9, -1.0],
  ["ashanti", 6.7, -1.6],
  ["accra", 5.6, -0.2],
];

function approxCoords(location: string): [number, number] | null {
  const lower = location.toLowerCase();
  for (const [key, lat, lon] of CITY_COORDS) {
    if (lower.includes(key)) return [lat, lon];
  }
  return null;
}

function levelFromScore(score: number): RiskLevel {
  if (score >= 75) return "CRITICAL";
  if (score >= 50) return "HIGH";
  if (score >= 25) return "MEDIUM";
  return "LOW";
}

export interface ScorerInput {
  product: Product;
  activeEvents: TrackingEvent[];
  archivedEvents: TrackingEvent[];
  certifications: Certification[];
}

/**
 * Compute the recall risk score for a product.
 * Pure function — no side effects, fully testable.
 */
export function computeRiskScore(input: ScorerInput): RiskScore {
  const { product, activeEvents, archivedEvents, certifications } = input;
  const allEvents = [...activeEvents, ...archivedEvents];
  const factors: RiskFactor[] = [];

  // F1 — No events at all
  if (allEvents.length === 0) {
    factors.push({
      id: "NO_EVENTS",
      label: "No tracking events",
      description: "This product has no recorded tracking events. Its provenance cannot be verified.",
      score: 30,
    });
  } else {
    // F2 — Missing HARVEST event
    const hasHarvest = allEvents.some((e) => e.eventType === "HARVEST");
    if (!hasHarvest) {
      factors.push({
        id: "MISSING_HARVEST",
        label: "Missing harvest event",
        description: "No HARVEST event found. The product's origin cannot be confirmed.",
        score: 20,
      });
    }

    // F3 — Events out of expected stage order
    const stageIndices = activeEvents
      .map((e) => STAGE_ORDER[e.eventType])
      .filter((i) => i !== undefined);
    let outOfOrder = false;
    for (let i = 1; i < stageIndices.length; i++) {
      if (stageIndices[i] < stageIndices[i - 1]) {
        outOfOrder = true;
        break;
      }
    }
    if (outOfOrder) {
      factors.push({
        id: "WRONG_ORDER",
        label: "Out-of-order lifecycle stages",
        description: "Events appear in an unexpected order (e.g. RETAIL before SHIPPING). This may indicate data tampering.",
        score: 25,
      });
    }

    // F4 — Suspicious location jump between consecutive events
    const sorted = [...activeEvents].sort((a, b) => a.timestamp - b.timestamp);
    let locationJump = false;
    for (let i = 1; i < sorted.length; i++) {
      const c1 = approxCoords(sorted[i - 1].location);
      const c2 = approxCoords(sorted[i].location);
      if (c1 && c2) {
        const km = haversineKm(c1[0], c1[1], c2[0], c2[1]);
        const hours = (sorted[i].timestamp - sorted[i - 1].timestamp) / 3_600_000;
        // Flag if >10,000 km in <6 hours (physically impossible by sea/land)
        if (km > 10_000 && hours < 6) {
          locationJump = true;
          break;
        }
      }
    }
    if (locationJump) {
      factors.push({
        id: "LOCATION_JUMP",
        label: "Impossible location jump",
        description: "Consecutive events show a geographic jump that is physically impossible given the elapsed time.",
        score: 20,
      });
    }

    // F5 — Long gap between consecutive events (>30 days)
    const MS_30_DAYS = 30 * 24 * 3_600_000;
    let longGap = false;
    for (let i = 1; i < sorted.length; i++) {
      if (sorted[i].timestamp - sorted[i - 1].timestamp > MS_30_DAYS) {
        longGap = true;
        break;
      }
    }
    if (longGap) {
      factors.push({
        id: "LONG_GAP",
        label: "Long gap between events",
        description: "More than 30 days elapsed between consecutive tracking events. The product may have been unmonitored.",
        score: 15,
      });
    }

    // F6 — Event actor not in authorizedActors
    const authorizedSet = new Set(product.authorizedActors.map((a) => a.toLowerCase()));
    const ownerLower = product.owner.toLowerCase();
    const unauthorizedActor = activeEvents.some(
      (e) =>
        e.actor.toLowerCase() !== ownerLower &&
        !authorizedSet.has(e.actor.toLowerCase())
    );
    if (unauthorizedActor) {
      factors.push({
        id: "UNAUTHORIZED_ACTOR",
        label: "Unauthorized event actor",
        description: "At least one event was recorded by an address that is not the owner or an authorized actor.",
        score: 25,
      });
    }

    // F7 — High archive ratio (>50% of total events archived)
    const totalCount = allEvents.length;
    const archivedCount = archivedEvents.length;
    if (totalCount > 0 && archivedCount / totalCount > 0.5) {
      factors.push({
        id: "HIGH_ARCHIVE_RATIO",
        label: "High archive ratio",
        description: `${archivedCount} of ${totalCount} events have been archived (>${Math.round((archivedCount / totalCount) * 100)}%). Excessive archiving may obscure the product history.`,
        score: 15,
      });
    }

    // F10 — Stale / empty metadata
    const staleMetadata = activeEvents.some((e) => {
      try {
        const parsed = JSON.parse(e.metadata);
        return !parsed || Object.keys(parsed).length === 0;
      } catch {
        return true;
      }
    });
    if (staleMetadata) {
      factors.push({
        id: "STALE_METADATA",
        label: "Empty event metadata",
        description: "One or more events carry empty or unparseable metadata, reducing auditability.",
        score: 10,
      });
    }
  }

  // F8 — No certifications
  if (certifications.length === 0) {
    factors.push({
      id: "NO_CERTIFICATIONS",
      label: "No certifications",
      description: "This product has no quality or compliance certifications on record.",
      score: 10,
    });
  }

  // F9 — Revoked certification
  const hasRevoked = certifications.some((c) => c.revoked);
  if (hasRevoked) {
    factors.push({
      id: "REVOKED_CERT",
      label: "Revoked certification",
      description: "At least one certification for this product has been revoked.",
      score: 15,
    });
  }

  const total = Math.min(
    100,
    factors.reduce((sum, f) => sum + f.score, 0)
  );

  return { total, level: levelFromScore(total), factors };
}
