import { describe, it, expect } from 'vitest';
import { calculateHealthScore, getHealthScoreColor, getHealthScoreBgColor } from './healthScore';
import type { Product, TrackingEvent } from '@/lib/types';

describe('healthScore', () => {
  const mockProduct: Product = {
    id: 'p1',
    name: 'Test Product',
    origin: 'Test Origin',
    owner: 'owner1',
    timestamp: 1000,
    authorizedActors: [],
  };

  it('returns poor score for product with no events', () => {
    const score = calculateHealthScore(mockProduct, []);
    expect(score.overallScore).toBeLessThan(40);
    expect(score.status).toBe('poor');
  });

  it('calculates freshness score based on event recency', () => {
    const now = Math.floor(Date.now() / 1000);
    const recentEvent: TrackingEvent = {
      productId: 'p1',
      location: 'Farm',
      actor: 'farmer',
      timestamp: now - 3 * 24 * 60 * 60, // 3 days ago
      eventType: 'HARVEST',
      metadata: '{}',
    };

    const score = calculateHealthScore(mockProduct, [recentEvent]);
    expect(score.freshnessScore).toBeGreaterThan(80);
  });

  it('calculates coverage score based on event type diversity', () => {
    const events: TrackingEvent[] = [
      {
        productId: 'p1',
        location: 'Farm',
        actor: 'farmer',
        timestamp: 1000,
        eventType: 'HARVEST',
        metadata: '{}',
      },
      {
        productId: 'p1',
        location: 'Factory',
        actor: 'processor',
        timestamp: 2000,
        eventType: 'PROCESSING',
        metadata: '{}',
      },
      {
        productId: 'p1',
        location: 'Port',
        actor: 'shipper',
        timestamp: 3000,
        eventType: 'SHIPPING',
        metadata: '{}',
      },
      {
        productId: 'p1',
        location: 'Store',
        actor: 'retailer',
        timestamp: 4000,
        eventType: 'RETAIL',
        metadata: '{}',
      },
    ];

    const score = calculateHealthScore(mockProduct, events);
    expect(score.coverageScore).toBe(100);
  });

  it('calculates verification score based on unique actors', () => {
    const events: TrackingEvent[] = [
      {
        productId: 'p1',
        location: 'Farm',
        actor: 'farmer1',
        timestamp: 1000,
        eventType: 'HARVEST',
        metadata: '{}',
      },
      {
        productId: 'p1',
        location: 'Factory',
        actor: 'processor1',
        timestamp: 2000,
        eventType: 'PROCESSING',
        metadata: '{}',
      },
      {
        productId: 'p1',
        location: 'Port',
        actor: 'shipper1',
        timestamp: 3000,
        eventType: 'SHIPPING',
        metadata: '{}',
      },
      {
        productId: 'p1',
        location: 'Store',
        actor: 'retailer1',
        timestamp: 4000,
        eventType: 'RETAIL',
        metadata: '{}',
      },
    ];

    const score = calculateHealthScore(mockProduct, events);
    expect(score.verificationScore).toBe(100);
  });

  it('returns excellent status for high score', () => {
    const now = Math.floor(Date.now() / 1000);
    const events: TrackingEvent[] = [
      {
        productId: 'p1',
        location: 'Farm',
        actor: 'farmer1',
        timestamp: now - 2 * 24 * 60 * 60,
        eventType: 'HARVEST',
        metadata: '{}',
      },
      {
        productId: 'p1',
        location: 'Factory',
        actor: 'processor1',
        timestamp: now - 1 * 24 * 60 * 60,
        eventType: 'PROCESSING',
        metadata: '{}',
      },
      {
        productId: 'p1',
        location: 'Port',
        actor: 'shipper1',
        timestamp: now - 12 * 60 * 60,
        eventType: 'SHIPPING',
        metadata: '{}',
      },
      {
        productId: 'p1',
        location: 'Store',
        actor: 'retailer1',
        timestamp: now - 1 * 60 * 60,
        eventType: 'RETAIL',
        metadata: '{}',
      },
    ];

    const score = calculateHealthScore(mockProduct, events);
    expect(score.status).toBe('excellent');
    expect(score.overallScore).toBeGreaterThanOrEqual(80);
  });

  it('returns correct color for status', () => {
    expect(getHealthScoreColor('excellent')).toBe('text-green-600');
    expect(getHealthScoreColor('good')).toBe('text-blue-600');
    expect(getHealthScoreColor('fair')).toBe('text-yellow-600');
    expect(getHealthScoreColor('poor')).toBe('text-red-600');
  });

  it('returns correct background color for status', () => {
    expect(getHealthScoreBgColor('excellent')).toBe('bg-green-50');
    expect(getHealthScoreBgColor('good')).toBe('bg-blue-50');
    expect(getHealthScoreBgColor('fair')).toBe('bg-yellow-50');
    expect(getHealthScoreBgColor('poor')).toBe('bg-red-50');
  });
});
