'use client';

import { useState, useEffect, useCallback } from 'react';
import {
  ShieldCheck,
  ShieldX,
  ShieldAlert,
  Clock,
  ExternalLink,
  ChevronDown,
  ChevronUp,
  RefreshCw,
  AlertCircle,
} from 'lucide-react';
import type { AttestationView, AttestationTrustLevel, AttestationType } from '@/lib/attestations';

// ── Trust level config ────────────────────────────────────────────────────────

const TRUST_LEVEL_CONFIG: Record<
  AttestationTrustLevel,
  { label: string; badgeClass: string; icon: React.ReactNode }
> = {
  verified: {
    label: 'Verified',
    badgeClass:
      'bg-green-50 text-green-700 border-green-300 dark:bg-green-900/20 dark:text-green-400 dark:border-green-800',
    icon: <ShieldCheck size={12} />,
  },
  trusted: {
    label: 'Trusted',
    badgeClass:
      'bg-blue-50 text-blue-700 border-blue-300 dark:bg-blue-900/20 dark:text-blue-400 dark:border-blue-800',
    icon: <ShieldCheck size={12} />,
  },
  community: {
    label: 'Community',
    badgeClass:
      'bg-amber-50 text-amber-700 border-amber-300 dark:bg-amber-900/20 dark:text-amber-400 dark:border-amber-800',
    icon: <ShieldAlert size={12} />,
  },
};

const ATTESTATION_TYPE_LABELS: Record<AttestationType, string> = {
  audit: 'Audit',
  certification: 'Certification',
  inspection: 'Inspection',
  compliance: 'Compliance',
  sustainability: 'Sustainability',
  custom: 'Custom',
};

// ── Sub-components ────────────────────────────────────────────────────────────

interface AttestationCardProps {
  attestation: AttestationView;
}

function AttestationCard({ attestation }: AttestationCardProps) {
  const [expanded, setExpanded] = useState(false);
  const trustConfig = TRUST_LEVEL_CONFIG[attestation.trustLevel];
  const isActive = attestation.status === 'active';
  const isRevoked = attestation.status === 'revoked';
  const isExpired = attestation.status === 'expired';

  return (
    <div
      className={`rounded-lg border p-4 transition-colors ${
        isRevoked || isExpired
          ? 'border-[var(--card-border)] opacity-60 bg-[var(--muted-bg)]'
          : 'border-[var(--card-border)] bg-[var(--card)]'
      }`}
    >
      {/* Header row */}
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-start gap-3 min-w-0">
          {/* Trust badge */}
          <span
            className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium border shrink-0 ${trustConfig.badgeClass}`}
          >
            {trustConfig.icon}
            {trustConfig.label}
          </span>

          <div className="min-w-0">
            <p className="text-sm font-semibold text-[var(--foreground)] truncate">
              {attestation.summary}
            </p>
            <p className="text-xs text-[var(--muted)] mt-0.5">
              {ATTESTATION_TYPE_LABELS[attestation.attestationType]} ·{' '}
              <span className="font-mono">{attestation.issuerName}</span>
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          {/* Status badge */}
          {isRevoked && (
            <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-red-100 text-red-600 border border-red-300 dark:bg-red-900/20 dark:text-red-400 dark:border-red-800">
              <ShieldX size={10} />
              Revoked
            </span>
          )}
          {isExpired && (
            <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-600 border border-gray-300 dark:bg-gray-800 dark:text-gray-400 dark:border-gray-700">
              <Clock size={10} />
              Expired
            </span>
          )}

          <button
            onClick={() => setExpanded((v) => !v)}
            className="p-1 rounded hover:bg-[var(--muted-bg)] text-[var(--muted)] transition-colors"
            aria-label={expanded ? 'Collapse attestation details' : 'Expand attestation details'}
            aria-expanded={expanded}
          >
            {expanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
          </button>
        </div>
      </div>

      {/* Expanded details */}
      {expanded && (
        <div className="mt-3 pt-3 border-t border-[var(--card-border)] space-y-2 text-xs text-[var(--muted)]">
          <div className="grid grid-cols-2 gap-x-4 gap-y-1">
            <span className="font-medium text-[var(--foreground)]">Attestation ID</span>
            <span className="font-mono truncate">{attestation.attestationId}</span>

            <span className="font-medium text-[var(--foreground)]">Issuer Address</span>
            <span className="font-mono truncate">{attestation.issuerAddress}</span>

            <span className="font-medium text-[var(--foreground)]">Issued</span>
            <span>{new Date(attestation.createdAt).toLocaleString()}</span>

            {attestation.expiresAt > 0 && (
              <>
                <span className="font-medium text-[var(--foreground)]">Expires</span>
                <span>{new Date(attestation.expiresAt).toLocaleString()}</span>
              </>
            )}

            {isRevoked && attestation.revokedAt > 0 && (
              <>
                <span className="font-medium text-[var(--foreground)]">Revoked</span>
                <span>{new Date(attestation.revokedAt).toLocaleString()}</span>
              </>
            )}

            {attestation.revocationReason && (
              <>
                <span className="font-medium text-[var(--foreground)]">Revocation Reason</span>
                <span>{attestation.revocationReason}</span>
              </>
            )}
          </div>

          {/* Signed reference */}
          <div>
            <p className="font-medium text-[var(--foreground)] mb-0.5">Signed Reference</p>
            <p className="font-mono break-all bg-[var(--muted-bg)] rounded px-2 py-1 text-[10px]">
              {attestation.signedReference}
            </p>
          </div>

          {/* Report URL */}
          {attestation.reportUrl && (
            <a
              href={attestation.reportUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1 text-violet-600 hover:text-violet-700 dark:text-violet-400 dark:hover:text-violet-300 transition-colors"
            >
              <ExternalLink size={12} />
              View full report
            </a>
          )}

          {/* Metadata */}
          {attestation.metadata && (
            <div>
              <p className="font-medium text-[var(--foreground)] mb-0.5">Metadata</p>
              <pre className="font-mono break-all bg-[var(--muted-bg)] rounded px-2 py-1 text-[10px] whitespace-pre-wrap">
                {attestation.metadata}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ── Main panel ────────────────────────────────────────────────────────────────

interface AttestationsPanelProps {
  productId: string;
  /** If true, show a compact summary suitable for product cards */
  compact?: boolean;
}

export function AttestationsPanel({ productId, compact = false }: AttestationsPanelProps) {
  const [attestations, setAttestations] = useState<AttestationView[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchAttestations = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(
        `/api/v1/attestations?productId=${encodeURIComponent(productId)}`,
      );
      if (!res.ok) {
        const data = await res.json().catch(() => ({}));
        throw new Error(data?.error?.message ?? `HTTP ${res.status}`);
      }
      const data = await res.json();
      setAttestations(data.attestations ?? []);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load attestations');
    } finally {
      setLoading(false);
    }
  }, [productId]);

  useEffect(() => {
    fetchAttestations();
  }, [fetchAttestations]);

  const active = attestations.filter((a) => a.status === 'active');
  const inactive = attestations.filter((a) => a.status !== 'active');

  // ── Compact mode (for product cards) ─────────────────────────────────────

  if (compact) {
    if (loading) return null;
    if (active.length === 0) return null;

    return (
      <div className="flex flex-wrap gap-1">
        {active.slice(0, 3).map((att) => {
          const trustConfig = TRUST_LEVEL_CONFIG[att.trustLevel];
          return (
            <span
              key={att.attestationId}
              title={`${att.summary} — ${att.issuerName}`}
              className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium border ${trustConfig.badgeClass}`}
            >
              {trustConfig.icon}
              {ATTESTATION_TYPE_LABELS[att.attestationType]}
            </span>
          );
        })}
        {active.length > 3 && (
          <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-[var(--muted-bg)] text-[var(--muted)] border border-[var(--card-border)]">
            +{active.length - 3} more
          </span>
        )}
      </div>
    );
  }

  // ── Full panel ────────────────────────────────────────────────────────────

  return (
    <section aria-label="Attestations">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-sm font-semibold text-[var(--foreground)]">
          Attestations
          {!loading && attestations.length > 0 && (
            <span className="ml-2 text-xs font-normal text-[var(--muted)]">
              ({active.length} active
              {inactive.length > 0 ? `, ${inactive.length} inactive` : ''})
            </span>
          )}
        </h3>
        <button
          onClick={fetchAttestations}
          disabled={loading}
          className="p-1 rounded hover:bg-[var(--muted-bg)] text-[var(--muted)] transition-colors disabled:opacity-50"
          aria-label="Refresh attestations"
          title="Refresh"
        >
          <RefreshCw size={14} className={loading ? 'animate-spin' : ''} />
        </button>
      </div>

      {loading && (
        <div className="space-y-2">
          {[1, 2].map((i) => (
            <div
              key={i}
              className="h-16 rounded-lg bg-[var(--muted-bg)] animate-pulse"
              aria-hidden="true"
            />
          ))}
        </div>
      )}

      {!loading && error && (
        <div className="flex items-center gap-2 p-3 rounded-lg bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-sm text-red-600 dark:text-red-400">
          <AlertCircle size={14} className="shrink-0" />
          {error}
        </div>
      )}

      {!loading && !error && attestations.length === 0 && (
        <p className="text-sm text-[var(--muted)]">
          No attestations registered for this product.
        </p>
      )}

      {!loading && !error && attestations.length > 0 && (
        <div className="space-y-2">
          {/* Active attestations first */}
          {active.map((att) => (
            <AttestationCard key={att.attestationId} attestation={att} />
          ))}

          {/* Revoked / expired in a collapsed section */}
          {inactive.length > 0 && (
            <details className="group">
              <summary className="cursor-pointer text-xs text-[var(--muted)] hover:text-[var(--foreground)] transition-colors list-none flex items-center gap-1 py-1">
                <ChevronDown
                  size={12}
                  className="group-open:rotate-180 transition-transform"
                />
                {inactive.length} revoked / expired
              </summary>
              <div className="mt-2 space-y-2">
                {inactive.map((att) => (
                  <AttestationCard key={att.attestationId} attestation={att} />
                ))}
              </div>
            </details>
          )}
        </div>
      )}
    </section>
  );
}
