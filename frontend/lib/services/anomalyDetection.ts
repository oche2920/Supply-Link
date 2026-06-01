/**
 * Issue #506: AI-assisted anomaly review assistant
 * Handles anomaly detection and AI-assisted analysis
 */

export type AnomalySeverity = 1 | 2 | 3 | 4;

export interface AnomalyReport {
  id: string;
  productId: string;
  anomalyType: string;
  severity: AnomalySeverity;
  description: string;
  suggestedActions: string;
  detectedAt: number;
  reviewed: boolean;
  reviewedBy: string;
  reviewedAt: number;
}

export interface AnomalyReportRequest {
  productId: string;
  anomalyType: string;
  severity: AnomalySeverity;
  description: string;
  suggestedActions: string;
}

export interface AnomalyReviewRequest {
  productId: string;
  reportId: string;
  analyst: string;
}

export interface AnomalySummary {
  totalAnomalies: number;
  criticalCount: number;
  highCount: number;
  mediumCount: number;
  lowCount: number;
  reviewedCount: number;
  pendingCount: number;
}

/**
 * Report an anomaly in a product's supply chain
 */
export async function reportAnomaly(request: AnomalyReportRequest): Promise<AnomalyReport> {
  const response = await fetch('/api/anomalies', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    throw new Error(`Failed to report anomaly: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Get all anomaly reports for a product
 */
export async function getAnomalyReports(productId: string): Promise<AnomalyReport[]> {
  const response = await fetch(`/api/anomalies?productId=${productId}`);

  if (!response.ok) {
    throw new Error(`Failed to fetch anomaly reports: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Mark an anomaly report as reviewed
 */
export async function reviewAnomaly(request: AnomalyReviewRequest): Promise<boolean> {
  const response = await fetch(`/api/anomalies/${request.reportId}/review`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    throw new Error(`Failed to review anomaly: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Get AI-generated summary and recommendations for anomalies
 */
export async function getAnomalySummary(productId: string): Promise<AnomalySummary> {
  const response = await fetch(`/api/anomalies/summary?productId=${productId}`);

  if (!response.ok) {
    throw new Error(`Failed to fetch anomaly summary: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Get AI-assisted analysis for a specific anomaly
 */
export async function getAnomalyAnalysis(
  productId: string,
  reportId: string,
): Promise<{ analysis: string; recommendations: string[] }> {
  const response = await fetch(`/api/anomalies/${reportId}/analysis?productId=${productId}`);

  if (!response.ok) {
    throw new Error(`Failed to fetch anomaly analysis: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Detect anomalies in a product's event history
 */
export async function detectAnomalies(productId: string): Promise<AnomalyReport[]> {
  const response = await fetch('/api/anomalies/detect', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ productId }),
  });

  if (!response.ok) {
    throw new Error(`Failed to detect anomalies: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Get severity label
 */
export function getSeverityLabel(severity: AnomalySeverity): string {
  const labels: Record<AnomalySeverity, string> = {
    1: 'Low',
    2: 'Medium',
    3: 'High',
    4: 'Critical',
  };
  return labels[severity];
}

/**
 * Get severity color
 */
export function getSeverityColor(severity: AnomalySeverity): string {
  const colors: Record<AnomalySeverity, string> = {
    1: 'yellow',
    2: 'orange',
    3: 'red',
    4: 'darkred',
  };
  return colors[severity];
}

/**
 * Filter anomalies by severity
 */
export function filterAnomaliesBySeverity(
  anomalies: AnomalyReport[],
  severity: AnomalySeverity,
): AnomalyReport[] {
  return anomalies.filter((a) => a.severity === severity);
}

/**
 * Filter unreviewed anomalies
 */
export function getUnreviewedAnomalies(anomalies: AnomalyReport[]): AnomalyReport[] {
  return anomalies.filter((a) => !a.reviewed);
}
