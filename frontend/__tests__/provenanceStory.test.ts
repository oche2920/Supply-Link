import { describe, it, expect } from 'vitest';
import {
  generateProvenanceStory,
  formatEventDate,
  formatEventTime,
  getEventIcon,
  getEventColor,
  getTimeElapsed,
} from '@/lib/services/provenanceStory';
import type { TrackingEvent } from '@/lib/types';

describe('provenanceStory', () => {
  const mockEvents: TrackingEvent[] = [
    {
      productId: 'p1',
      location: 'Farm A',
      actor: 'farmer1',
      timestamp: 1000,
      eventType: 'HARVEST',
      metadata: '{}',
    },
    {
      productId: 'p1',
      location: 'Factory B',
      actor: 'processor1',
      timestamp: 2000,
      eventType: 'PROCESSING',
      metadata: '{}',
    },
    {
      productId: 'p1',
      location: 'Port C',
      actor: 'shipper1',
      timestamp: 3000,
      eventType: 'SHIPPING',
      metadata: '{}',
    },
    {
      productId: 'p1',
      location: 'Store D',
      actor: 'retailer1',
      timestamp: 4000,
      eventType: 'RETAIL',
      metadata: '{}',
    },
  ];

  it('generates story segments from events', () => {
    const story = generateProvenanceStory(mockEvents);
    expect(story).toHaveLength(4);
    expect(story[0].eventType).toBe('HARVEST');
    expect(story[1].eventType).toBe('PROCESSING');
    expect(story[2].eventType).toBe('SHIPPING');
    expect(story[3].eventType).toBe('RETAIL');
  });

  it('generates appropriate narratives for each event', () => {
    const story = generateProvenanceStory(mockEvents);
    expect(story[0].narrative).toContain('Farm A');
    expect(story[0].narrative).toContain('farmer1');
    expect(story[1].narrative).toContain('Factory B');
    expect(story[2].narrative).toContain('Port C');
    expect(story[3].narrative).toContain('Store D');
  });

  it('sorts events by timestamp', () => {
    const unsortedEvents = [mockEvents[3], mockEvents[0], mockEvents[2], mockEvents[1]];
    const story = generateProvenanceStory(unsortedEvents);
    expect(story[0].timestamp).toBe(1000);
    expect(story[1].timestamp).toBe(2000);
    expect(story[2].timestamp).toBe(3000);
    expect(story[3].timestamp).toBe(4000);
  });

  it('formats event date correctly', () => {
    const timestamp = 1609459200; // 2021-01-01
    const formatted = formatEventDate(timestamp);
    expect(formatted).toContain('Jan');
    expect(formatted).toContain('2021');
  });

  it('formats event time correctly', () => {
    const timestamp = 1609459200;
    const formatted = formatEventTime(timestamp);
    expect(formatted).toMatch(/\d{2}:\d{2}/);
  });

  it('returns correct icon for event type', () => {
    expect(getEventIcon('HARVEST')).toBe('Sprout');
    expect(getEventIcon('PROCESSING')).toBe('Factory');
    expect(getEventIcon('SHIPPING')).toBe('Truck');
    expect(getEventIcon('RETAIL')).toBe('Store');
  });

  it('returns correct color for event type', () => {
    expect(getEventColor('HARVEST')).toContain('green');
    expect(getEventColor('PROCESSING')).toContain('blue');
    expect(getEventColor('SHIPPING')).toContain('purple');
    expect(getEventColor('RETAIL')).toContain('orange');
  });

  it('calculates time elapsed correctly', () => {
    const elapsed = getTimeElapsed(1000, 86400 + 1000); // 1 day later
    expect(elapsed).toContain('1 day');
  });

  it('handles metadata parsing', () => {
    const eventWithMetadata: TrackingEvent = {
      productId: 'p1',
      location: 'Farm',
      actor: 'farmer',
      timestamp: 1000,
      eventType: 'HARVEST',
      metadata: JSON.stringify({ temperature: 25, humidity: 60 }),
    };

    const story = generateProvenanceStory([eventWithMetadata]);
    expect(story[0].metadata).toEqual({ temperature: 25, humidity: 60 });
  });
});
