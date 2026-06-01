'use client';

import { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import {
  certifyEventTimestamp,
  getEventTimestampCerts,
  revokeEventTimestampCert,
  verifyTimestampCert,
  type EventTimestampCert,
} from '@/lib/services/timestampCertification';

interface TimestampCertificationPanelProps {
  productId: string;
  eventStableId: string;
  certifier: string;
}

export function TimestampCertificationPanel({
  productId,
  eventStableId,
  certifier,
}: TimestampCertificationPanelProps) {
  const [certs, setCerts] = useState<EventTimestampCert[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleCertify = async () => {
    setLoading(true);
    setError(null);
    try {
      await certifyEventTimestamp({
        productId,
        eventStableId,
        certifier,
      });
      const updated = await getEventTimestampCerts(productId);
      setCerts(updated);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to certify timestamp');
    } finally {
      setLoading(false);
    }
  };

  const handleRevoke = async (certId: string) => {
    setLoading(true);
    setError(null);
    try {
      await revokeEventTimestampCert(productId, certId);
      const updated = await getEventTimestampCerts(productId);
      setCerts(updated);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to revoke certification');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Event Timestamp Certification</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <Button onClick={handleCertify} disabled={loading}>
          {loading ? 'Certifying...' : 'Certify Timestamp'}
        </Button>

        {error && <div className="text-red-600 text-sm">{error}</div>}

        <div className="space-y-2">
          {certs.map((cert) => (
            <div key={cert.id} className="flex items-center justify-between p-2 border rounded">
              <div className="text-sm">
                <div className="font-medium">
                  {verifyTimestampCert(cert) ? '✓ Valid' : '✗ Revoked'}
                </div>
                <div className="text-gray-600">
                  Certified: {new Date(cert.issuedAt * 1000).toLocaleString()}
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
