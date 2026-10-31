import { describe, expect, it } from 'vitest';
// @ts-expect-error CJS module import
import { ARCHIVE_SHA256, assertArchiveDigest } from '../scripts/sidecar-integrity.cjs';

describe('sidecar integrity digests', () => {
  it('every supported archive has a pinned SHA-256 digest', () => {
    expect(Object.keys(ARCHIVE_SHA256).length).toBe(6);
    for (const [name, digest] of Object.entries(ARCHIVE_SHA256 as Record<string, string>)) {
      expect(digest).toMatch(/^[a-f0-9]{64}$/);
      expect(() => assertArchiveDigest(name, digest)).not.toThrow();
    }
  });

  it('rejects unknown and modified archives', () => {
    expect(() => assertArchiveDigest('unknown.zip', 'a'.repeat(64))).toThrow(/Integrity check failed/);
    expect(() => assertArchiveDigest('wintun-0.14.1.zip', 'a'.repeat(64))).toThrow(/Integrity check failed/);
  });
});
