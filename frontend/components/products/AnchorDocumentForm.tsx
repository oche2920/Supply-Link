'use client';

import { useState, useRef } from 'react';
import { Loader2, Paperclip, CheckCircle2, AlertTriangle } from 'lucide-react';
import { useStore } from '@/lib/state/store';
import { contractClient } from '@/lib/stellar/contract';
import { hashFile } from '@/lib/crypto/documentHash';

interface Props {
  productId: string;
  onAnchored?: () => void;
}

export function AnchorDocumentForm({ productId, onAnchored }: Props) {
  const walletAddress = useStore((s) => s.walletAddress);
  const fileRef = useRef<HTMLInputElement>(null);

  const [label, setLabel] = useState('');
  const [file, setFile] = useState<File | null>(null);
  const [hash, setHash] = useState('');
  const [status, setStatus] = useState<'idle' | 'hashing' | 'submitting' | 'done' | 'error'>(
    'idle',
  );
  const [error, setError] = useState('');

  async function handleFileChange(e: React.ChangeEvent<HTMLInputElement>) {
    const f = e.target.files?.[0] ?? null;
    setFile(f);
    setHash('');
    if (!f) return;
    setStatus('hashing');
    try {
      const h = await hashFile(f);
      setHash(h);
      setStatus('idle');
    } catch {
      setError('Failed to compute file hash.');
      setStatus('error');
    }
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!walletAddress) {
      setError('Connect your wallet first.');
      return;
    }
    if (!hash) {
      setError('Select a file first.');
      return;
    }
    if (!label.trim()) {
      setError('Enter a document label.');
      return;
    }

    setStatus('submitting');
    setError('');
    try {
      await contractClient.anchorDocumentHash(productId, label.trim(), hash, walletAddress);
      setStatus('done');
      setLabel('');
      setFile(null);
      setHash('');
      if (fileRef.current) fileRef.current.value = '';
      onAnchored?.();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Transaction failed.');
      setStatus('error');
    }
  }

  const busy = status === 'hashing' || status === 'submitting';

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-[var(--foreground)] mb-1">
          Document label
        </label>
        <input
          type="text"
          value={label}
          onChange={(e) => setLabel(e.target.value)}
          placeholder="e.g. Certificate of Origin"
          maxLength={256}
          className="w-full rounded-lg border border-[var(--card-border)] bg-[var(--background)] px-3 py-2 text-sm text-[var(--foreground)] focus:outline-none focus:ring-2 focus:ring-[var(--primary)]"
          disabled={busy}
        />
      </div>

      <div>
        <label className="block text-sm font-medium text-[var(--foreground)] mb-1">
          Document file
        </label>
        <div className="flex items-center gap-3">
          <label className="flex items-center gap-2 cursor-pointer rounded-lg border border-dashed border-[var(--card-border)] px-4 py-2 text-sm text-[var(--muted)] hover:border-[var(--primary)] transition-colors">
            <Paperclip size={14} />
            {file ? file.name : 'Choose file…'}
            <input
              ref={fileRef}
              type="file"
              className="sr-only"
              onChange={handleFileChange}
              disabled={busy}
            />
          </label>
          {status === 'hashing' && (
            <Loader2 size={14} className="animate-spin text-[var(--muted)]" />
          )}
        </div>
        {hash && (
          <p className="mt-1 font-mono text-xs text-[var(--muted)] break-all">SHA-256: {hash}</p>
        )}
      </div>

      {error && (
        <p className="flex items-center gap-1.5 text-sm text-red-600">
          <AlertTriangle size={14} /> {error}
        </p>
      )}

      {status === 'done' && (
        <p className="flex items-center gap-1.5 text-sm text-green-600">
          <CheckCircle2 size={14} /> Document hash anchored on-chain.
        </p>
      )}

      <button
        type="submit"
        disabled={busy || !hash || !label.trim()}
        className="flex items-center gap-2 rounded-lg bg-[var(--primary)] px-4 py-2 text-sm font-medium text-white disabled:opacity-50 hover:opacity-90 transition-opacity"
      >
        {status === 'submitting' && <Loader2 size={14} className="animate-spin" />}
        Anchor on-chain
      </button>
    </form>
  );
}
