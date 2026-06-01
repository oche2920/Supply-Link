/**
 * Product health score based on event freshness and verification coverage.
 * Closes #486
 */

import type { Product, TrackingEvent } from '@/lib/types';

export interface HealthScoreBreakdown {
  freshnessScore: number; // 0-100: based on how recent the last event is
  coverageScore: number; // 0-100: based on event type diversity
  verificationScore: number; // 0-100: based on number of verifications
  overallScore: number; // 0-100: weighted average
  status: 'excellent' | 'good' | 'fair' | 'poor';
}

const FRESHNESS_THRESHOLD_DAYS = 30; // Events older than this reduce freshness score
const COVERAGE_WEIGHT = 0.3;
const FRESHNESS_WEIGHT = 0.4;
const VERIFICATION_WEIGHT = 0.3;

/**
 * Calculate freshness score based on how recent the last event is
 * 100 = event within 7 days, decreases linearly to 0 at 90+ days
 */
function calculateFreshnessScore(lastEventTimestamp: number): number {
  const now = Math.floor(Date.now() / 1000);
  const daysSinceLastEvent = (now - lastEventTimestamp) / (24 * 60 * 60);

  if (daysSinceLastEvent <= 7) return 100;
  if (daysSinceLastEvent >= 90) return 0;

  // Linear interpolation between 7 and 90 days
  return Math.max(0, 100 - ((daysSinceLastEvent - 7) / (90 - 7)) * 100);
}

/**
 * Calculate coverage score based on event type diversity
 * 100 = all 4 event types present, 0 = no events
 */
function calculateCoverageScore(events: TrackingEvent[]): number {
  if (events.length === 0) return 0;

  const eventTypes = new Set(events.map((e) => e.eventType));
  const expectedTypes = 4; // HARVEST, PROCESSING, SHIPPING, RETAIL

  return Math.round((eventTypes.size / expectedTypes) * 100);
}

/**
 * Calculate verification score based on number of unique actors
 * More actors = more verification points
 */
function calculateVerificationScore(events: TrackingEvent[]): number {
  if (events.length === 0) return 0;

  const uniqueActors = new Set(events.map((e) => e.actor));
  const actorCount = uniqueActors.size;

  // Score increases with more actors: 1 actor = 25%, 2 = 50%, 3 = 75%, 4+ = 100%
  return Math.min(100, (actorCount / 4) * 100);
}

/**
 * Calculate overall health score for a product
 */
export function calculateHealthScore(
  product: Product,
  events: TrackingEvent[],
): HealthScoreBreakdown {
  const freshnessScore = calculateFreshnessScore(
    events.length > 0 ? Math.max(...events.map((e) => e.timestamp)) : product.timestamp,
  );
  const coverageScore = calculateCoverageScore(events);
  const verificationScore = calculateVerificationScore(events);

  const overallScore = Math.round(
    freshnessScore * FRESHNESS_WEIGHT +
      coverageScore * COVERAGE_WEIGHT +
      verificationScore * VERIFICATION_WEIGHT,
  );

  let status: 'excellent' | 'good' | 'fair' | 'poor';
  if (overallScore >= 80) status = 'excellent';
  else if (overallScore >= 60) status = 'good';
  else if (overallScore >= 40) status = 'fair';
  else status = 'poor';

  return {
    freshnessScore,
    coverageScore,
    verificationScore,
    overallScore,
    status,
  };
}

/**
 * Get color for health score status
 */
export function getHealthScoreColor(status: string): string {
  switch (status) {
    case 'excellent':
      return 'text-green-600';
    case 'good':
      return 'text-blue-600';
    case 'fair':
      return 'text-yellow-600';
    case 'poor':
      return 'text-red-600';
    default:
      return 'text-gray-600';
  }
}

/**
 * Get background color for health score status
 */
export function getHealthScoreBgColor(status: string): string {
  switch (status) {
    case 'excellent':
      return 'bg-green-50';
    case 'good':
      return 'bg-blue-50';
    case 'fair':
      return 'bg-yellow-50';
    case 'poor':
      return 'bg-red-50';
    default:
      return 'bg-gray-50';
  }
}
