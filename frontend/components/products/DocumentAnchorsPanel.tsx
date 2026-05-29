'use client';

import { useState, useRef } from 'react';
import { CheckCircle2, XCircle, Paperclip, Loader2 } from 'lucide-react';
import type { DocumentAnchor } from '@/lib/types';
import { hashFile } from '@/lib/crypto/documentHash';

interface Props {
  anchors: DocumentAnchor[];
}

interface VerifyState {
  status: 'idle' | 'hashing' | 'match' | 'mismatch';
  computedHash?: string;
}

export function DocumentAnchorsPanel({ anchors }: Props) {
  const [verifyStates, setVerifyStates] = useState<Record<number, VerifyState>>({});
  const fileRefs = useRef<Record<number, HTMLInputElement | null>>({});

  async function handleVerify(index: number, anchorHash: string, file: File) {
    setVerifyStates((prev) => ({ ...prev, [index]: { status: 'hashing' } }));
    try {
      const computed = await hashFile(file);
      const match = computed === anchorHash;
      setVerifyStates((prev) => ({
        ...prev,
        [index]: { status: match ? 'match' : 'mismatch', computedHash: computed },
      }));
    } catch {
      setVerifyStates((prev) => ({ ...prev, [index]: { status: 'mismatch' } }));
    }
  }

  if (anchors.length === 0) {
    return <p className="text-sm text-[var(--muted)]">No documents anchored yet.</p>;
  }

  return (
    <ul className="space-y-4">
      {anchors.map((anchor, i) => {
        const state = verifyStates[i] ?? { status: 'idle' };
        return (
          <li
            key={i}
            className="rounded-lg border border-[var(--card-border)] bg-[var(--background)] p-4"
          >
            <div className="flex items-start justify-between gap-4">
              <div className="min-w-0">
                <p className="text-sm font-medium text-[var(--foreground)] truncate">
                  {anchor.label}
                </p>
                <p className="font-mono text-xs text-[var(--muted)] break-all mt-0.5">
                  {anchor.hash}
                </p>
                <p className="text-xs text-[var(--muted)] mt-1">
                  Anchored by <span className="font-mono">{anchor.anchoredBy.slice(0, 8)}…</span>
                  {' · '}
                  {new Date(anchor.anchoredAt * 1000).toLocaleString()}
                </p>
              </div>
            </div>

            {/* Verify against local file */}
            <div className="mt-3 flex items-center gap-3 flex-wrap">
              <label className="flex items-center gap-1.5 cursor-pointer rounded border border-dashed border-[var(--card-border)] px-3 py-1.5 text-xs text-[var(--muted)] hover:border-[var(--primary)] transition-colors">
                <Paperclip size={12} />
                Verify local file
                <input
                  type="file"
                  className="sr-only"
                  ref={(el) => {
                    fileRefs.current[i] = el;
                  }}
                  onChange={(e) => {
                    const f = e.target.files?.[0];
                    if (f) handleVerify(i, anchor.hash, f);
                  }}
                />
              </label>

              {state.status === 'hashing' && (
                <span className="flex items-center gap-1 text-xs text-[var(--muted)]">
                  <Loader2 size={12} className="animate-spin" /> Computing…
                </span>
              )}
              {state.status === 'match' && (
                <span className="flex items-center gap-1 text-xs text-green-600 font-medium">
                  <CheckCircle2 size={12} /> Hash matches — document is authentic
                </span>
              )}
              {state.status === 'mismatch' && (
                <span className="flex items-center gap-1 text-xs text-red-600 font-medium">
                  <XCircle size={12} /> Hash mismatch — document may have been modified
                </span>
              )}
            </div>
          </li>
        );
      })}
    </ul>
  );
}
