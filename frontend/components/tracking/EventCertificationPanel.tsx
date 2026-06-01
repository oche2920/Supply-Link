'use client';

import { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import {
  certifyEvent,
  getEventCertifications,
  revokeEventCertification,
  verifyEventCertification,
  type EventCertification,
} from '@/lib/services/eventCertification';

interface EventCertificationPanelProps {
  productId: string;
  eventStableId: string;
  certifierId: string;
  certType: string;
}

export function EventCertificationPanel({
  productId,
  eventStableId,
  certifierId,
  certType,
}: EventCertificationPanelProps) {
  const [certifications, setCertifications] = useState<EventCertification[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [metadata, setMetadata] = useState('{}');

  const handleCertify = async () => {
    setLoading(true);
    setError(null);
    try {
      await certifyEvent({
        productId,
        eventStableId,
        certType,
        certifierId,
        metadata,
      });

      const updated = await getEventCertifications(productId);
      setCertifications(updated);
      setMetadata('{}');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to certify event');
    } finally {
      setLoading(false);
    }
  };

  const handleRevoke = async (certId: string) => {
    setLoading(true);
    setError(null);
    try {
      await revokeEventCertification(productId, certId);
      const updated = await getEventCertifications(productId);
      setCertifications(updated);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to revoke certification');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Event Certification</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          <label className="text-sm font-medium">Metadata (JSON)</label>
          <textarea
            value={metadata}
            onChange={(e) => setMetadata(e.target.value)}
            className="w-full p-2 border rounded text-sm font-mono"
            rows={3}
            placeholder='{"key": "value"}'
          />
        </div>

        <Button onClick={handleCertify} disabled={loading}>
          {loading ? 'Certifying...' : 'Certify Event'}
        </Button>

        {error && <div className="text-red-600 text-sm">{error}</div>}

        <div className="space-y-2">
          {certifications.map((cert) => (
            <div key={cert.id} className="flex items-center justify-between p-2 border rounded">
              <div className="text-sm">
                <div className="font-medium">
                  {verifyEventCertification(cert) ? '✓ Valid' : '✗ Revoked'}
                </div>
                <div className="text-gray-600">Type: {cert.certType}</div>
                <div className="text-gray-600">
                  Issued: {new Date(cert.issuedAt * 1000).toLocaleString()}
                </div>
              </div>
              {!cert.revoked && (
                <Button
                  variant="destructive"
                  size="sm"
                  onClick={() => handleRevoke(cert.id)}
                  disabled={loading}
                >
                  Revoke
                </Button>
              )}
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
