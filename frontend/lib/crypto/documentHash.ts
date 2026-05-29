/**
 * Document hash utilities for off-chain document anchoring (#460).
 *
 * Uses the Web Crypto API (available in all modern browsers and Node ≥18)
 * to compute SHA-256 digests of arbitrary file/document bytes.
 */

/**
 * Compute the SHA-256 hex digest of a File or Blob.
 * Returns a 64-character lowercase hex string.
 */
export async function hashFile(file: File | Blob): Promise<string> {
  const buffer = await file.arrayBuffer();
  return hashBuffer(buffer);
}

/**
 * Compute the SHA-256 hex digest of an ArrayBuffer.
 * Returns a 64-character lowercase hex string.
 */
export async function hashBuffer(buffer: ArrayBuffer): Promise<string> {
  const digest = await globalThis.crypto.subtle.digest('SHA-256', buffer);
  return bufferToHex(digest);
}

function bufferToHex(buffer: ArrayBuffer): string {
  return Array.from(new Uint8Array(buffer))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}
