'use client';

import { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import {
  getAnomalyReports,
  reviewAnomaly,
  detectAnomalies,
  getAnomalySummary,
  getSeverityLabel,
  getSeverityColor,
  getUnreviewedAnomalies,
  type AnomalyReport,
  type AnomalySummary,
} from '@/lib/services/anomalyDetection';

interface AnomalyReviewPanelProps {
  productId: string;
  analyst: string;
}

export function AnomalyReviewPanel({ productId, analyst }: AnomalyReviewPanelProps) {
  const [anomalies, setAnomalies] = useState<AnomalyReport[]>([]);
  const [summary, setSummary] = useState<AnomalySummary | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedAnomaly, setSelectedAnomaly] = useState<AnomalyReport | null>(null);

  useEffect(() => {
    loadAnomalies();
  }, [productId]);

  const loadAnomalies = async () => {
    setLoading(true);
    setError(null);
    try {
      const reports = await getAnomalyReports(productId);
      setAnomalies(reports);

      const summaryData = await getAnomalySummary(productId);
      setSummary(summaryData);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load anomalies');
    } finally {
      setLoading(false);
    }
  };

  const handleDetect = async () => {
    setLoading(true);
    setError(null);
    try {
      const detected = await detectAnomalies(productId);
      setAnomalies(detected);
      const summaryData = await getAnomalySummary(productId);
      setSummary(summaryData);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to detect anomalies');
    } finally {
      setLoading(false);
    }
  };

  const handleReview = async (reportId: string) => {
    setLoading(true);
    setError(null);
    try {
      await reviewAnomaly({
        productId,
        reportId,
        analyst,
      });
      await loadAnomalies();
      setSelectedAnomaly(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to review anomaly');
    } finally {
      setLoading(false);
    }
  };

  const unreviewedCount = getUnreviewedAnomalies(anomalies).length;

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader>
          <CardTitle>Anomaly Detection & Review</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex gap-2">
            <Button onClick={handleDetect} disabled={loading}>
              {loading ? 'Detecting...' : 'Detect Anomalies'}
            </Button>
            <Button onClick={loadAnomalies} disabled={loading} variant="outline">
              Refresh
            </Button>
          </div>

          {error && <div className="text-red-600 text-sm">{error}</div>}

          {summary && (
            <div className="grid grid-cols-2 gap-2 text-sm">
              <div className="p-2 bg-gray-100 rounded">
                <div className="font-medium">Total</div>
                <div className="text-lg">{summary.totalAnomalies}</div>
              </div>
              <div className="p-2 bg-red-100 rounded">
                <div className="font-medium">Critical</div>
                <div className="text-lg">{summary.criticalCount}</div>
              </div>
              <div className="p-2 bg-orange-100 rounded">
                <div className="font-medium">High</div>
                <div className="text-lg">{summary.highCount}</div>
              </div>
              <div className="p-2 bg-yellow-100 rounded">
                <div className="font-medium">Pending Review</div>
                <div className="text-lg">{unreviewedCount}</div>
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      {selectedAnomaly && (
        <Card>
          <CardHeader>
            <CardTitle>Anomaly Details</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div>
              <div className="font-medium">Type: {selectedAnomaly.anomalyType}</div>
              <div className="text-sm">
                Severity:{' '}
                <span
                  style={{
                    color: getSeverityColor(selectedAnomaly.severity),
                    fontWeight: 'bold',
                  }}
                >
                  {getSeverityLabel(selectedAnomaly.severity)}
                </span>
              </div>
            </div>

            <div>
              <div className="font-medium text-sm mb-1">Description</div>
              <div className="text-sm text-gray-700">{selectedAnomaly.description}</div>
            </div>

            <div>
              <div className="font-medium text-sm mb-1">Suggested Actions</div>
              <div className="text-sm text-gray-700 whitespace-pre-wrap">
                {selectedAnomaly.suggestedActions}
              </div>
            </div>

            <div className="flex gap-2">
              {!selectedAnomaly.reviewed && (
                <Button onClick={() => handleReview(selectedAnomaly.id)} disabled={loading}>
                  Mark as Reviewed
                </Button>
              )}
              <Button onClick={() => setSelectedAnomaly(null)} variant="outline">
                Close
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardHeader>
          <CardTitle>Anomaly Reports</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            {anomalies.map((anomaly) => (
              <div
                key={anomaly.id}
                className="flex items-center justify-between p-2 border rounded cursor-pointer hover:bg-gray-50"
                onClick={() => setSelectedAnomaly(anomaly)}
              >
                <div className="text-sm">
                  <div className="font-medium">{anomaly.anomalyType}</div>
                  <div className="text-gray-600">
                    {anomaly.reviewed ? '✓ Reviewed' : '⚠ Pending Review'}
                  </div>
                </div>
                <div
                  className="px-2 py-1 rounded text-white text-xs font-medium"
                  style={{ backgroundColor: getSeverityColor(anomaly.severity) }}
                >
                  {getSeverityLabel(anomaly.severity)}
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
