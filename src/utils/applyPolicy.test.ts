import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  classifySetting, classifySettingsUpdate, createApplier, validateSettingsForApply,
  type ApplyOutcome,
} from './applyPolicy';
import type { Settings } from './store';

describe('classification', () => {
  it('restarts for config-affecting keys', () => {
    for (const key of ['mixedPort', 'apiPort', 'tunMtu', 'sniffHttp', 'primaryDns', 'dnsMode', 'wifiSharing', 'multiplex', 'finalOutbound', 'bypassLan'] as const) {
      expect(classifySetting(key)).toBe('restart');
    }
  });
  it('keeps app-only keys live and proxyMode deferred', () => {
    expect(classifySetting('closeToTray')).toBe('live');
    expect(classifySetting('theme')).toBe('live');
    expect(classifySetting('autostart')).toBe('live');
    expect(classifySetting('geoLookupEnabled')).toBe('live');
    expect(classifySetting('proxyMode')).toBe('defer');
  });
  it('combines: restart > defer > live', () => {
    expect(classifySettingsUpdate({ closeToTray: false, mixedPort: 1 })).toBe('restart');
    expect(classifySettingsUpdate({ closeToTray: false, proxyMode: 'system' })).toBe('defer');
    expect(classifySettingsUpdate({ closeToTray: false })).toBe('live');
  });
});

describe('validateSettingsForApply', () => {
  const ok = { httpPort: 1, socksPort: 2, mixedPort: 7892, apiPort: 9090, tunMtu: 1500 } as Settings;
  it('accepts valid values', () => expect(validateSettingsForApply(ok)).toBeNull());
  it('rejects bad ports and mtu', () => {
    expect(validateSettingsForApply({ ...ok, mixedPort: 0 })).toMatch(/mixedPort/);
    expect(validateSettingsForApply({ ...ok, apiPort: 70000 })).toMatch(/apiPort/);
    expect(validateSettingsForApply({ ...ok, tunMtu: 100 })).toMatch(/tunMtu/);
  });
});

describe('applier', () => {
  type S = { v: number };
  let connected: boolean;
  let running: boolean;
  let reloads: number;
  let reloadImpl: (n: number) => Promise<void>;
  let restored: S[];
  let reports: [ApplyOutcome, string | undefined][];
  let validation: string | null;

  const make = () => createApplier<S>({
    debounceMs: 50,
    isConnected: () => connected,
    isRunning: async () => running,
    merge: (existing) => existing,
    validate: () => validation,
    reload: async () => { reloads += 1; await reloadImpl(reloads); },
    restore: async (b) => { restored.push(b); },
    report: (o, d) => { reports.push([o, d]); },
  });

  beforeEach(() => {
    vi.useFakeTimers();
    connected = true; running = true; reloads = 0; restored = []; reports = []; validation = null;
    reloadImpl = async () => {};
  });
  afterEach(() => vi.useRealTimers());

  it('debounces many edits into one restart', async () => {
    const a = make();
    a.schedule({ v: 1 }); a.schedule({ v: 2 }); a.schedule({ v: 3 });
    await vi.advanceTimersByTimeAsync(60);
    expect(reloads).toBe(1);
    expect(reports.map((r) => r[0])).toEqual(['applied']);
    expect(restored).toEqual([]);
  });

  it('skips when no longer connected', async () => {
    const a = make();
    a.schedule({ v: 1 });
    connected = false;
    await vi.advanceTimersByTimeAsync(60);
    expect(reloads).toBe(0);
    expect(reports.map((r) => r[0])).toEqual(['skipped']);
    expect(a.hasPending()).toBe(false);
  });

  it('keeps pending and does not restart on invalid values', async () => {
    const a = make();
    validation = 'mixedPort must be between 1 and 65535';
    a.schedule({ v: 1 });
    await vi.advanceTimersByTimeAsync(60);
    expect(reloads).toBe(0);
    expect(reports[0]).toEqual(['invalid', validation]);
    expect(a.hasPending()).toBe(true);
    validation = null;
    a.schedule({ v: 2 });
    await vi.advanceTimersByTimeAsync(60);
    expect(reloads).toBe(1);
    expect(restored).toEqual([]);
  });

  it('restores the first baseline and keeps the old session when validation fails before stop', async () => {
    const a = make();
    reloadImpl = async () => { throw new Error('validation failed'); };
    a.schedule({ v: 1 });
    await vi.advanceTimersByTimeAsync(60);
    expect(restored).toEqual([{ v: 1 }]);
    expect(reloads).toBe(1);
    expect(reports[0][0]).toBe('kept-running');
  });

  it('restores and recovers when restart fails after stop', async () => {
    const a = make();
    reloadImpl = async (n) => {
      if (n === 1) { running = false; throw new Error('start failed'); }
      running = true;
    };
    a.schedule({ v: 1 });
    await vi.advanceTimersByTimeAsync(60);
    expect(restored).toEqual([{ v: 1 }]);
    expect(reloads).toBe(2);
    expect(reports[0][0]).toBe('recovered');
  });

  it('reports recovery failure when the recovery restart also fails', async () => {
    const a = make();
    reloadImpl = async () => { running = false; throw new Error('boom'); };
    a.schedule({ v: 1 });
    await vi.advanceTimersByTimeAsync(60);
    expect(restored).toEqual([{ v: 1 }]);
    expect(reloads).toBe(2);
    expect(reports[0][0]).toBe('recovery-failed');
    expect(reports[0][1]).toContain('recovery');
  });

  it('treats a reload that leaves the proxy stopped as a failure', async () => {
    const a = make();
    reloadImpl = async (n) => { running = n > 1; };
    a.schedule({ v: 1 });
    await vi.advanceTimersByTimeAsync(60);
    expect(restored).toEqual([{ v: 1 }]);
    expect(reports[0][0]).toBe('recovered');
  });

  it('re-arms for edits made while a restart is in flight', async () => {
    const a = make();
    let release: () => void = () => {};
    reloadImpl = (n) => (n === 1 ? new Promise<void>((r) => { release = r; }) : Promise.resolve());
    a.schedule({ v: 1 });
    await vi.advanceTimersByTimeAsync(60);
    a.schedule({ v: 2 });
    release();
    await vi.advanceTimersByTimeAsync(200);
    expect(reloads).toBe(2);
    expect(reports.map((r) => r[0])).toEqual(['applied', 'applied']);
  });
});
