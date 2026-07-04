/**
 * Generates a RFC4122 v4 compliant UUID.
 * Uses `crypto.randomUUID()` when available, falling back to a Math.random-based
 * generator in non-secure contexts (e.g. custom protocols in some WebViews).
 */
export function safeRandomUUID(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }
  // Math.random fallback (RFC 4122 v4 compliant representation)
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}
