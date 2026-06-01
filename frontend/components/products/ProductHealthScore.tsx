'use client';

import {
  calculateHealthScore,
  getHealthScoreColor,
  getHealthScoreBgColor,
} from '@/lib/services/healthScore';
import type { Product, TrackingEvent } from '@/lib/types';

interface ProductHealthScoreProps {
  product: Product;
  events: TrackingEvent[];
  compact?: boolean;
}

export function ProductHealthScore({ product, events, compact = false }: ProductHealthScoreProps) {
  const score = calculateHealthScore(product, events);

  if (compact) {
    return (
      <div
        className={`inline-flex items-center gap-2 px-3 py-1 rounded-full ${getHealthScoreBgColor(score.status)}`}
      >
        <span className={`text-sm font-semibold ${getHealthScoreColor(score.status)}`}>
          {score.overallScore}%
        </span>
        <span className={`text-xs font-medium ${getHealthScoreColor(score.status)}`}>
          {score.status}
        </span>
      </div>
    );
  }

  return (
    <div className={`rounded-lg p-4 border ${getHealthScoreBgColor(score.status)}`}>
      <div className="flex items-center justify-between mb-4">
        <h3 className="font-semibold text-gray-900">Product Health Score</h3>
        <div className={`text-2xl font-bold ${getHealthScoreColor(score.status)}`}>
          {score.overallScore}%
        </div>
      </div>

      <div className="space-y-3">
        <div>
          <div className="flex justify-between items-center mb-1">
            <span className="text-sm text-gray-700">Freshness</span>
            <span className="text-sm font-medium text-gray-900">
              {Math.round(score.freshnessScore)}%
            </span>
          </div>
          <div className="w-full bg-gray-200 rounded-full h-2">
            <div
              className="bg-blue-500 h-2 rounded-full"
              style={{ width: `${score.freshnessScore}%` }}
            />
          </div>
        </div>

        <div>
          <div className="flex justify-between items-center mb-1">
            <span className="text-sm text-gray-700">Coverage</span>
            <span className="text-sm font-medium text-gray-900">
              {Math.round(score.coverageScore)}%
            </span>
          </div>
          <div className="w-full bg-gray-200 rounded-full h-2">
            <div
              className="bg-green-500 h-2 rounded-full"
              style={{ width: `${score.coverageScore}%` }}
            />
          </div>
        </div>

        <div>
          <div className="flex justify-between items-center mb-1">
            <span className="text-sm text-gray-700">Verification</span>
            <span className="text-sm font-medium text-gray-900">
              {Math.round(score.verificationScore)}%
            </span>
          </div>
          <div className="w-full bg-gray-200 rounded-full h-2">
            <div
              className="bg-purple-500 h-2 rounded-full"
              style={{ width: `${score.verificationScore}%` }}
            />
          </div>
        </div>
      </div>

      <div className="mt-4 pt-4 border-t border-gray-200">
        <p className={`text-sm font-medium ${getHealthScoreColor(score.status)}`}>
          Status: <span className="capitalize">{score.status}</span>
        </p>
      </div>
    </div>
  );
}
