'use client';

import { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import {
  notarizeProvenance,
  getProvenanceNotarizations,
  revokeProvenanceNotarization,
  verifyProvenanceNotarization,
  calculateProofHash,
  type ProvenanceNotarization,
} from '@/lib/services/provenanceNotarization';

interface ProvenanceNotarizationPanelProps {
  productId: string;
  notary: string;
}

export function ProvenanceNotarizationPanel({
  productId,
  notary,
}: ProvenanceNotarizationPanelProps) {
  const [notarizations, setNotarizations] = useState<ProvenanceNotarization[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleNotarize = async () => {
    setLoading(true);
    setError(null);
    try {
      const proofHash = await calculateProofHash(productId);
      const expiresAt = Math.floor(Date.now() / 1000) + 365 * 24 * 60 * 60; // 1 year

      await notarizeProvenance({
        productId,
        proofHash,
        notary,
        expiresAt,
      });

      const updated = await getProvenanceNotarizations(productId);
      setNotarizations(updated);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to notarize provenance');
    } finally {
      setLoading(false);
    }
  };

  const handleRevoke = async (notarizationId: string) => {
    setLoading(true);
    setError(null);
    try {
      await revokeProvenanceNotarization(productId, notarizationId);
      const updated = await getProvenanceNotarizations(productId);
      setNotarizations(updated);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to revoke notarization');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Provenance Notarization</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <Button onClick={handleNotarize} disabled={loading}>
          {loading ? 'Notarizing...' : 'Notarize Provenance'}
        </Button>

        {error && <div className="text-red-600 text-sm">{error}</div>}

        <div className="space-y-2">
          {notarizations.map((notarization) => (
            <div
              key={notarization.id}
              className="flex items-center justify-between p-2 border rounded"
            >
              <div className="text-sm">
                <div className="font-medium">
                  {verifyProvenanceNotarization(notarization) ? '✓ Valid' : '✗ Expired/Revoked'}
                </div>
                <div className="text-gray-600">
                  Notarized: {new Date(notarization.notarizedAt * 1000).toLocaleString()}
                </div>
                <div className="text-gray-600 text-xs font-mono">
                  {notarization.proofHash.substring(0, 16)}...
                </div>
              </div>
              {!notarization.revoked && (
                <Button
                  variant="destructive"
                  size="sm"
                  onClick={() => handleRevoke(notarization.id)}
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
