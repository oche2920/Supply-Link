import { describe, it, expect } from 'vitest';
import {
  detectLifecycleGaps,
  hasCriticalGaps,
  getGapSummary,
} from '@/lib/services/lifecycleGapDetector';
import type { TrackingEvent } from '@/lib/types';

describe('lifecycleGapDetector', () => {
  it('detects critical gap when no events exist', () => {
    const analysis = detectLifecycleGaps([]);
    expect(analysis.hasGaps).toBe(true);
    expect(analysis.completionPercentage).toBe(0);
    expect(hasCriticalGaps(analysis)).toBe(true);
  });

  it('detects missing event types', () => {
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
    ];

    const analysis = detectLifecycleGaps(events);
    expect(analysis.hasGaps).toBe(true);
    expect(analysis.gaps.some((g) => g.expectedEventType === 'SHIPPING')).toBe(true);
    expect(analysis.gaps.some((g) => g.expectedEventType === 'RETAIL')).toBe(true);
  });

  it('detects time gaps between events', () => {
    const thirtyDaysInSeconds = 30 * 24 * 60 * 60;
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
        timestamp: 1000 + thirtyDaysInSeconds + 1000, // 30+ days later
        eventType: 'PROCESSING',
        metadata: '{}',
      },
    ];

    const analysis = detectLifecycleGaps(events);
    expect(analysis.gaps.some((g) => g.description.includes('Large time gap'))).toBe(true);
  });

  it('calculates completion percentage correctly', () => {
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

    const analysis = detectLifecycleGaps(events);
    expect(analysis.hasGaps).toBe(false);
    expect(analysis.completionPercentage).toBe(100);
  });

  it('generates appropriate gap summary', () => {
    const events: TrackingEvent[] = [
      {
        productId: 'p1',
        location: 'Farm',
        actor: 'farmer',
        timestamp: 1000,
        eventType: 'HARVEST',
        metadata: '{}',
      },
    ];

    const analysis = detectLifecycleGaps(events);
    const summary = getGapSummary(analysis);
    expect(summary).toContain('warning');
  });
});
