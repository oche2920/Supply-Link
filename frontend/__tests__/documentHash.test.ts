import { describe, it, expect, beforeAll } from 'vitest';
import { hashBuffer, hashFile } from '@/lib/crypto/documentHash';

// Web Crypto is available in jsdom/happy-dom via globalThis.crypto.subtle
describe('documentHash', () => {
  it('hashBuffer returns a 64-char hex string', async () => {
    const buf = new TextEncoder().encode('hello world').buffer;
    const hex = await hashBuffer(buf);
    expect(hex).toHaveLength(64);
    expect(hex).toMatch(/^[0-9a-f]{64}$/);
  });

  it('hashBuffer is deterministic', async () => {
    const buf = new TextEncoder().encode('supply-link').buffer;
    const h1 = await hashBuffer(buf);
    const h2 = await hashBuffer(buf);
    expect(h1).toBe(h2);
  });

  it('hashBuffer produces different hashes for different inputs', async () => {
    const a = await hashBuffer(new TextEncoder().encode('doc-a').buffer);
    const b = await hashBuffer(new TextEncoder().encode('doc-b').buffer);
    expect(a).not.toBe(b);
  });

  it('hashFile returns the same hash as hashBuffer for the same bytes', async () => {
    const bytes = new TextEncoder().encode('test document content');
    const file = new File([bytes], 'test.txt', { type: 'text/plain' });
    const fromFile = await hashFile(file);
    const fromBuffer = await hashBuffer(bytes.buffer);
    expect(fromFile).toBe(fromBuffer);
  });

  it('known SHA-256: empty string', async () => {
    const buf = new ArrayBuffer(0);
    const hex = await hashBuffer(buf);
    // SHA-256 of empty input
    expect(hex).toBe('e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855');
  });
});
