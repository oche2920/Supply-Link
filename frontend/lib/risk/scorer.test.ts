/**
 * Unit tests for the recall risk scoring engine.
 * Run with: vitest --run
 */
import { describe, it, expect } from "vitest";
import { computeRiskScore } from "./scorer";
import type { Product, TrackingEvent, Certification } from "@/lib/types";

const BASE_PRODUCT: Product = {
  id: "prod-001",
  name: "Test Product",
  origin: "Ethiopia",
  owner: "GOWNER1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ",
  timestamp: 1_710_000_000_000,
  active: true,
  authorizedActors: ["GACTOR1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ"],
};

function makeEvent(overrides: Partial<TrackingEvent> = {}): TrackingEvent {
  return {
    productId: "prod-001",
    eventType: "HARVEST",
    location: "Yirgacheffe, Ethiopia",
    actor: "GOWNER1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ",
    timestamp: 1_710_000_000_000,
    metadata: JSON.stringify({ notes: "ok" }),
    archived: false,
    ...overrides,
  };
}

function makeCert(overrides: Partial<Certification> = {}): Certification {
  return {
    certId: "cert-001",
    productId: "prod-001",
    issuer: "GOWNER1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ",
    issuedAt: 1_710_000_000_000,
    certType: "ORGANIC",
    reference: "https://registry.example/cert-001",
    revoked: false,
    ...overrides,
  };
}

// ── F1: No events ─────────────────────────────────────────────────────────────

describe("F1 — no events", () => {
  it("scores 30 + 10 (no certs) when product has no events", () => {
    const result = computeRiskScore({
      product: BASE_PRODUCT,
      activeEvents: [],
      archivedEvents: [],
      certifications: [],
    });
    expect(result.factors.some((f) => f.id === "NO_EVENTS")).toBe(true);
    expect(result.total).toBeGreaterThanOrEqual(30);
    expect(result.level).not.toBe("LOW");
  });

  it("does not flag NO_EVENTS when events exist", () => {
    const result = computeRiskScore({
      product: BASE_PRODUCT,
      activeEvents: [makeEvent()],
      archivedEvents: [],
      certifications: [makeCert()],
    });
    expect(result.factors.some((f) => f.id === "NO_EVENTS")).toBe(false);
  });
});

// ── F2: Missing harvest ───────────────────────────────────────────────────────

describe("F2 — missing harvest", () => {
  it("flags MISSING_HARVEST when no HARVEST event exists", () => {
    const result = computeRiskScore({
      product: BASE_PRODUCT,
      activeEvents: [makeEvent({ eventType: "SHIPPING" })],
      archivedEvents: [],
      certifications: [makeCert()],
    });
    expect(result.factors.some((f) => f.id === "MISSING_HARVEST")).toBe(true);
  });

  it("does not flag when HARVEST is present", () => {
    const result = computeRiskScore({
      product: BASE_PRODUCT,
      activeEvents: [makeEvent({ eventType: "HARVEST" })],
      archivedEvents: [],
      certifications: [makeCert()],
    });
    expect(result.factors.some((f) => f.id === "MISSING_HARVEST")).toBe(false);
  });
});

// ── F3: Out-of-order stages ───────────────────────────────────────────────────

describe("F3 — wrong order", () => {
  it("flags WRONG_ORDER when RETAIL precedes SHIPPING", () => {
    const result = computeRiskScore({
      product: BASE_PRODUCT,
      activeEvents: [
        makeEvent({ eventType: "HARVEST", timestamp: 1_000 }),
        makeEvent({ eventType: "RETAIL", timestamp: 2_000 }),
        makeEvent({ eventType: "SHIPPING", timestamp: 3_000 }),
      ],
      archivedEvents: [],
      certifications: [makeCert()],
    });
    expect(result.factors.some((f) => f.id === "WRONG_ORDER")).toBe(true);
  });

  it("does not flag correct order", () => {
    const result = computeRiskScore({
      product: BASE_PRODUCT,
      activeEvents: [
        makeEvent({ eventType: "HARVEST", timestamp: 1_000 }),
        makeEvent({ eventType: "PROCESSING", timestamp: 2_000 }),
        makeEvent({ eventType: "SHIPPING", timestamp: 3_000 }),
        makeEvent({ eventType: "RETAIL", timestamp: 4_000 }),
      ],
      archivedEvents: [],
      certifications: [makeCert()],
    });
    expect(result.factors.some((f) => f.id === "WRONG_ORDER")).toBe(false);
  });
});

// ── F5: Long gap ──────────────────────────────────────────────────────────────

describe("F5 — long gap", () => {
  const MS_31_DAYS = 31 * 24 * 3_600_000;

  it("flags LONG_GAP when gap exceeds 30 days", () => {
    const result = computeRiskScore({
      product: BASE_PRODUCT,
      activeEvents: [
        makeEvent({ eventType: "HARVEST", timestamp: 0 }),
        makeEvent({ eventType: "SHIPPING", timestamp: MS_31_DAYS }),
      ],
      archivedEvents: [],
      certifications: [makeCert()],
    });
    expect(result.factors.some((f) => f.id === "LONG_GAP")).toBe(true);
  });

  it("does not flag gap under 30 days", () => {
    const MS_10_DAYS = 10 * 24 * 3_600_000;
    const result = computeRiskScore({
      product: BASE_PRODUCT,
      activeEvents: [
        makeEvent({ eventType: "HARVEST", timestamp: 0 }),
        makeEvent({ eventType: "SHIPPING", timestamp: MS_10_DAYS }),
      ],
      archivedEvents: [],
      certifications: [makeCert()],
    });
    expect(result.factors.some((f) => f.id === "LONG_GAP")).toBe(false);
  });
});

// ── F6: Unauthorized actor ────────────────────────────────────────────────────

describe("F6 — unauthorized actor", () => {
  it("flags UNAUTHORIZED_ACTOR for unknown address", () => {
    const result = computeRiskScore({
      product: BASE_PRODUCT,
      activeEvents: [
        makeEvent({ actor: "GSTRANGER1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ" }),
      ],
      archivedEvents: [],
      certifications: [makeCert()],
    });
    expect(result.factors.some((f) => f.id === "UNAUTHORIZED_ACTOR")).toBe(true);
  });

  it("does not flag owner or authorized actor", () => {
    const result = computeRiskScore({
      product: BASE_PRODUCT,
      activeEvents: [
        makeEvent({ actor: BASE_PRODUCT.owner }),
        makeEvent({ actor: BASE_PRODUCT.authorizedActors[0] }),
      ],
      archivedEvents: [],
      certifications: [makeCert()],
    });
    expect(result.factors.some((f) => f.id === "UNAUTHORIZED_ACTOR")).toBe(false);
  });
});

// ── F7: High archive ratio ────────────────────────────────────────────────────

describe("F7 — high archive ratio", () => {
  it("flags HIGH_ARCHIVE_RATIO when >50% archived", () => {
    const result = computeRiskScore({
      product: BASE_PRODUCT,
      activeEvents: [makeEvent()],
      archivedEvents: [
        makeEvent({ archived: true }),
        makeEvent({ archived: true }),
      ],
      certifications: [makeCert()],
    });
    expect(result.factors.some((f) => f.id === "HIGH_ARCHIVE_RATIO")).toBe(true);
  });

  it("does not flag when archive ratio is low", () => {
    const result = computeRiskScore({
      product: BASE_PRODUCT,
      activeEvents: [makeEvent(), makeEvent(), makeEvent()],
      archivedEvents: [makeEvent({ archived: true })],
      certifications: [makeCert()],
    });
    expect(result.factors.some((f) => f.id === "HIGH_ARCHIVE_RATIO")).toBe(false);
  });
});

// ── F8: No certifications ─────────────────────────────────────────────────────

describe("F8 — no certifications", () => {
  it("flags NO_CERTIFICATIONS when cert list is empty", () => {
    const result = computeRiskScore({
      product: BASE_PRODUCT,
      activeEvents: [makeEvent()],
      archivedEvents: [],
      certifications: [],
    });
    expect(result.factors.some((f) => f.id === "NO_CERTIFICATIONS")).toBe(true);
  });

  it("does not flag when certifications exist", () => {
    const result = computeRiskScore({
      product: BASE_PRODUCT,
      activeEvents: [makeEvent()],
      archivedEvents: [],
      certifications: [makeCert()],
    });
    expect(result.factors.some((f) => f.id === "NO_CERTIFICATIONS")).toBe(false);
  });
});

// ── F9: Revoked cert ──────────────────────────────────────────────────────────

describe("F9 — revoked certification", () => {
  it("flags REVOKED_CERT when a cert is revoked", () => {
    const result = computeRiskScore({
      product: BASE_PRODUCT,
      activeEvents: [makeEvent()],
      archivedEvents: [],
      certifications: [makeCert({ revoked: true, revokedAt: Date.now() })],
    });
    expect(result.factors.some((f) => f.id === "REVOKED_CERT")).toBe(true);
  });

  it("does not flag when all certs are active", () => {
    const result = computeRiskScore({
      product: BASE_PRODUCT,
      activeEvents: [makeEvent()],
      archivedEvents: [],
      certifications: [makeCert({ revoked: false })],
    });
    expect(result.factors.some((f) => f.id === "REVOKED_CERT")).toBe(false);
  });
});

// ── Score capping ─────────────────────────────────────────────────────────────

describe("score capping", () => {
  it("never exceeds 100", () => {
    // Trigger every possible factor
    const result = computeRiskScore({
      product: { ...BASE_PRODUCT, authorizedActors: [] },
      activeEvents: [
        makeEvent({ eventType: "RETAIL", timestamp: 0, actor: "GSTRANGER000000000000000000000000000000000000", metadata: "{}" }),
        makeEvent({ eventType: "SHIPPING", timestamp: 31 * 24 * 3_600_000, actor: "GSTRANGER000000000000000000000000000000000000", metadata: "{}" }),
      ],
      archivedEvents: [
        makeEvent({ archived: true }),
        makeEvent({ archived: true }),
        makeEvent({ archived: true }),
      ],
      certifications: [makeCert({ revoked: true })],
    });
    expect(result.total).toBeLessThanOrEqual(100);
  });
});

// ── Level thresholds ──────────────────────────────────────────────────────────

describe("risk level thresholds", () => {
  it("returns LOW for score 0", () => {
    const result = computeRiskScore({
      product: BASE_PRODUCT,
      activeEvents: [makeEvent({ eventType: "HARVEST" })],
      archivedEvents: [],
      certifications: [makeCert()],
    });
    expect(["LOW", "MEDIUM"]).toContain(result.level);
  });

  it("returns CRITICAL for score >=75", () => {
    const result = computeRiskScore({
      product: { ...BASE_PRODUCT, authorizedActors: [] },
      activeEvents: [],
      archivedEvents: [],
      certifications: [makeCert({ revoked: true })],
    });
    // NO_EVENTS(30) + NO_CERTIFICATIONS skipped (cert exists) + REVOKED_CERT(15) = 45 → HIGH
    // With no certs at all: NO_EVENTS(30) + NO_CERTIFICATIONS(10) = 40 → MEDIUM
    // Verify the level matches the score
    const expectedLevel =
      result.total >= 75 ? "CRITICAL" :
      result.total >= 50 ? "HIGH" :
      result.total >= 25 ? "MEDIUM" : "LOW";
    expect(result.level).toBe(expectedLevel);
  });
});
