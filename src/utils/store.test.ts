import { describe, it, expect } from 'vitest';
import {
  DEFAULT_SETTINGS,
  generateApiSecret,
  migrateApiSettings,
} from './store';

describe('Clash API Hardening & Settings Migration', () => {
  it('default settings enforce secure Clash API baseline', () => {
    // 1. CORS wildcard must not be default
    expect(DEFAULT_SETTINGS.apiCors).toBe(false);
    // 2. API is not exposed by default
    expect(DEFAULT_SETTINGS.apiEnabled).toBe(false);
    expect(DEFAULT_SETTINGS.apiPort).toBe(9090);
  });

  it('generateApiSecret produces non-empty secure random tokens', () => {
    const token1 = generateApiSecret();
    const token2 = generateApiSecret();
    expect(typeof token1).toBe('string');
    expect(token1.length).toBeGreaterThanOrEqual(16);
    expect(typeof token2).toBe('string');
    expect(token2.length).toBeGreaterThanOrEqual(16);
    expect(token1).not.toBe(token2);
  });

  it('migrateApiSettings generates a secure token for empty or missing secret', () => {
    // Empty string
    const res1 = migrateApiSettings({ apiSecret: '' });
    expect(res1.migrated).toBe(true);
    expect(res1.settings.apiSecret.length).toBeGreaterThanOrEqual(16);

    // Whitespace only
    const res2 = migrateApiSettings({ apiSecret: '   ' });
    expect(res2.migrated).toBe(true);
    expect(res2.settings.apiSecret.length).toBeGreaterThanOrEqual(16);

    // Missing / undefined
    const res3 = migrateApiSettings({});
    expect(res3.migrated).toBe(true);
    expect(res3.settings.apiSecret.length).toBeGreaterThanOrEqual(16);
  });

  it('migrateApiSettings preserves existing valid secret without re-migrating', () => {
    const existingSecret = 'custom-user-token-12345';
    const res = migrateApiSettings({ apiSecret: existingSecret });
    expect(res.migrated).toBe(false);
    expect(res.settings.apiSecret).toBe(existingSecret);
  });

  it('migrateApiSettings preserves other settings while migrating secret', () => {
    const res = migrateApiSettings({
      httpPort: 1234,
      socksPort: 5678,
      apiPort: 9091,
      apiEnabled: true,
      apiSecret: '',
    });
    expect(res.migrated).toBe(true);
    expect(res.settings.httpPort).toBe(1234);
    expect(res.settings.socksPort).toBe(5678);
    expect(res.settings.apiPort).toBe(9091);
    expect(res.settings.apiEnabled).toBe(true);
    expect(res.settings.apiSecret.length).toBeGreaterThanOrEqual(16);
  });

  it('migrateApiSettings keeps apiCors false by default and preserves explicit setting', () => {
    // Default fallback
    const resDefault = migrateApiSettings({});
    expect(resDefault.settings.apiCors).toBe(false);

    // Explicitly true
    const resTrue = migrateApiSettings({ apiCors: true });
    expect(resTrue.settings.apiCors).toBe(true);

    // Explicitly false
    const resFalse = migrateApiSettings({ apiCors: false });
    expect(resFalse.settings.apiCors).toBe(false);
  });
});
