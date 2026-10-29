// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { isTauriEnvironment, loadAppStore } from './tauri';

describe('browser preview Tauri fallback', () => {
  beforeEach(() => {
    delete (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

  it('detects whether Tauri internals are present', () => {
    expect(isTauriEnvironment()).toBe(false);
    (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = undefined;
    expect(isTauriEnvironment()).toBe(false);
    (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    expect(isTauriEnvironment()).toBe(true);
  });

  it('keeps browser stores isolated by file and usable without native IPC', async () => {
    const settings = await loadAppStore('preview-settings.json');
    const profiles = await loadAppStore('preview-profiles.json');
    await settings.set('theme', 'light');
    await settings.save();
    await settings.reload({ ignoreDefaults: true });
    expect(await settings.get('theme')).toBe('light');
    expect(await profiles.get('theme')).toBeUndefined();
    expect(await settings.entries()).toEqual([['theme', 'light']]);
    expect(await settings.delete('theme')).toBe(true);
    expect(await settings.get('theme')).toBeUndefined();
  });

  it('does not try loading a native store in browser mode', async () => {
    const load = vi.spyOn((await import('@tauri-apps/plugin-store')).Store, 'load');
    await loadAppStore('preview-no-ipc.json');
    expect(load).not.toHaveBeenCalled();
    load.mockRestore();
  });
});
