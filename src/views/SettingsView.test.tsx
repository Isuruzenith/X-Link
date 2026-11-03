// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { ask } from '@tauri-apps/plugin-dialog';
import { useSettingsStore } from '../stores/settingsStore';
import { useConnectionStore } from '../stores/connectionStore';
import { useToastStore } from '../stores/toastStore';
import { storeHelper } from '../utils/store';
import { SettingsView } from './SettingsView';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));
vi.mock('@tauri-apps/plugin-dialog', () => ({ ask: vi.fn() }));
vi.mock('../utils/store', async (importOriginal) => {
  const original = await importOriginal<typeof import('../utils/store')>();
  return { ...original, storeHelper: {
    ...original.storeHelper,
    reloadSettings: vi.fn().mockResolvedValue(undefined),
    reloadProfiles: vi.fn().mockResolvedValue([]),
    reloadRouting: vi.fn().mockResolvedValue(undefined),
    getSettings: vi.fn().mockResolvedValue(original.DEFAULT_SETTINGS),
    getProfiles: vi.fn().mockResolvedValue([]),
    getActiveProfileId: vi.fn().mockResolvedValue(null),
    getRoutingRules: vi.fn().mockResolvedValue([]),
    getRuleSets: vi.fn().mockResolvedValue([]),
    migrateFromLegacy: vi.fn().mockResolvedValue(undefined),
  } };
});

afterEach(cleanup);

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(ask).mockResolvedValue(true);
  Object.assign(navigator, {
    clipboard: {
      writeText: vi.fn().mockResolvedValue(undefined),
    },
  });
  useToastStore.setState({ toasts: [] });
  useSettingsStore.setState({
    isElevated: true,
    appVersion: '0.8.0',
    conflictingPorts: [],
    portStatuses: { 2080: 'free', 2081: 'free', 9090: 'free' },
    settings: {
      ...useSettingsStore.getState().settings,
      proxyMode: 'system',
      apiEnabled: true,
      apiPort: 9090,
      apiSecret: 'test-secret',
    },
  });
  useConnectionStore.setState({ isConnected: true });
  vi.mocked(storeHelper.reloadProfiles).mockResolvedValue([]);
});

describe('SettingsView Diagnostics and Simulation', () => {
  it('switches to the diagnostics section when secondary nav item is clicked', () => {
    render(<SettingsView />);
    expect(screen.queryByText('System Health & Bundle')).toBeNull();

    fireEvent.click(screen.getByText('Diagnostics'));
    expect(screen.getByText('System Health & Bundle')).toBeTruthy();
    expect(screen.getByText('Routing Rule & DNS Simulation')).toBeTruthy();
  });

  it('generates, displays, and copies sanitized system diagnostics report', async () => {
    const mockReport = {
      timestamp: 1730000000,
      appVersion: '0.8.0',
      os: { platform: 'windows', arch: 'x86_64', isElevated: true },
      sidecar: {
        bundledTestedVersion: '1.10.0',
        cachedVersion: '1.10.0',
        isCompatible: true,
        compatibilityError: null,
      },
      proxy: { status: 'running', effectiveMode: 'system', tunSupported: true },
      ports: {
        mixedPort: 2080,
        socksPort: 2081,
        httpPort: 2082,
        apiPort: 9090,
        portStatuses: { 2080: 'bound' },
      },
      systemProxy: { managedByXlink: true },
      profiles: {
        profileCount: 1,
        activeProfileType: 'remote',
        nodeCountsByType: { vmess: 2 },
      },
      settings: { proxyMode: 'system' },
      recentLogs: ['[INFO] Core initialized', '[INFO] Token: [REDACTED]'],
    };

    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'export_system_diagnostics') return mockReport;
      return null;
    });

    render(<SettingsView />);
    fireEvent.click(screen.getByText('Diagnostics'));

    const generateBtn = screen.getByText('Generate Diagnostics');
    fireEvent.click(generateBtn);

    await waitFor(() => {
      expect(vi.mocked(invoke)).toHaveBeenCalledWith('export_system_diagnostics');
      expect(screen.getByText(/Diagnostics Preview/i)).toBeTruthy();
      expect(screen.getByText(/\[REDACTED\]/)).toBeTruthy();
    });

    const copyBtn = screen.getByText('Copy JSON');
    fireEvent.click(copyBtn);

    await waitFor(() => {
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
        JSON.stringify(mockReport, null, 2)
      );
    });
  });

  it('runs routing simulation and renders evaluated outbound results', async () => {
    const mockSimulationResult = {
      destination: '192.168.1.1',
      destinationType: 'ip',
      matchedOutbound: 'direct',
      matchedRuleId: null,
      matchedRuleType: null,
      matchReason: 'Private IPv4 address matches LAN bypass rule',
      isLanBypass: true,
      dnsStrategy: 'prefer_ipv4',
    };

    vi.mocked(invoke).mockImplementation(async (command, args) => {
      if (command === 'simulate_routing') {
        expect(args).toEqual({ destination: '192.168.1.1' });
        return mockSimulationResult;
      }
      return null;
    });

    const { container } = render(<SettingsView />);
    fireEvent.click(screen.getByText('Diagnostics'));

    const destInput = container.querySelector('md-outlined-text-field')!;
    Object.defineProperty(destInput, 'value', { value: '192.168.1.1', writable: true, configurable: true });
    fireEvent.input(destInput);

    const buttons = container.querySelectorAll('md-filled-button');
    const simulateBtn = Array.from(buttons).find((b) => b.textContent?.includes('Simulate'))!;
    fireEvent.click(simulateBtn);

    await waitFor(() => {
      expect(vi.mocked(invoke)).toHaveBeenCalledWith('simulate_routing', {
        destination: '192.168.1.1',
      });
      expect(screen.getByText('DIRECT')).toBeTruthy();
      expect(screen.getByText(/Private IPv4 address matches LAN bypass rule/i)).toBeTruthy();
      expect(screen.getByText(/LAN \/ Direct Bypass/i)).toBeTruthy();
    });
  });

  it('inspects an archive and requires disconnect before restoring selected sections', async () => {
    const archive = {
      metadata: { schemaVersion: 1, appVersion: '0.8.0', exportedAt: 1, platform: 'windows' },
      profileConfigs: { p1: '{"outbounds":[{"tag":"node","type":"vmess"}]}' },
      profiles: { profiles: [{ id: 'p1', name: 'Saved profile', type: 'manual' }] },
      settings: { proxyMode: 'system' },
    };
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'validate_backup_archive') return {
        profileCount: 1, profileNames: ['Saved profile'], hasSettings: true, ruleCount: 0,
        ruleSetCount: 0, appVersion: '0.8.0', exportedAt: 1,
      };
      if (command === 'import_config_backup') return { message: 'Restored. Restart the app.' };
      return null;
    });
    const { container } = render(<SettingsView />);
    fireEvent.click(screen.getByText('Diagnostics'));
    const fileInput = container.querySelector('input[type="file"]') as HTMLInputElement;
    const file = new File([JSON.stringify(archive)], 'backup.json', { type: 'application/json' });
    Object.defineProperty(file, 'text', { value: async () => JSON.stringify(archive) });
    fireEvent.change(fileInput, { target: { files: [file] } });
    await waitFor(() => expect(screen.getByText('Profiles: Saved profile')).toBeTruthy());
    expect(vi.mocked(invoke)).toHaveBeenCalledWith('validate_backup_archive', { archive });
    const restore = screen.getByText('Restore selected sections').closest('md-filled-button')!;
    expect(restore.hasAttribute('disabled')).toBe(true);
    useConnectionStore.setState({ isConnected: false, connectionStatus: 'disconnected' });
    await waitFor(() => expect(restore.hasAttribute('disabled')).toBe(false));
    fireEvent.click(restore);
    await waitFor(() => expect(ask).toHaveBeenCalledWith(expect.stringContaining('Restore profiles, settings'), expect.anything()));
    await waitFor(() => expect(vi.mocked(invoke)).toHaveBeenCalledWith('import_config_backup', {
      archive, options: { restoreSettings: true, restoreProfiles: true, restoreRouting: false },
    }));
    await waitFor(() => expect(storeHelper.reloadProfiles).toHaveBeenCalled());
  });

  it('reports a committed restore accurately when the frontend cache reload fails', async () => {
    const archive = {
      metadata: { schemaVersion: 1, appVersion: '0.8.0', exportedAt: 1, platform: 'windows' },
      settings: { proxyMode: 'system' }, profiles: null, profileConfigs: {}, routing: null,
    };
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'validate_backup_archive') return {
        profileCount: 0, profileNames: [], hasSettings: true, ruleCount: 0,
        ruleSetCount: 0, appVersion: '0.8.0', exportedAt: 1,
      };
      if (command === 'import_config_backup') return { message: 'Restored. Restart the app.' };
      return null;
    });
    vi.mocked(storeHelper.reloadSettings).mockRejectedValueOnce(new Error('cache unavailable'));
    useConnectionStore.setState({ isConnected: false, connectionStatus: 'disconnected' });
    const { container } = render(<SettingsView />);
    fireEvent.click(screen.getByText('Diagnostics'));
    const file = new File([JSON.stringify(archive)], 'backup.json');
    Object.defineProperty(file, 'text', { value: async () => JSON.stringify(archive) });
    fireEvent.change(container.querySelector('input[type="file"]')!, { target: { files: [file] } });
    await waitFor(() => expect(screen.getByText('Backup from X-Link 0.8.0')).toBeTruthy());
    fireEvent.click(screen.getByText('Restore selected sections'));
    await waitFor(() => expect(vi.mocked(invoke)).toHaveBeenCalledWith('import_config_backup', expect.anything()));
    await waitFor(() => expect(screen.getByRole('alert').textContent).toContain('Backup restored, but the app could not reload its state'));
    expect(screen.getByRole('alert').textContent).toContain('Restart the app before reconnecting');
  });

  it('does not overwrite local state when restore confirmation is cancelled', async () => {
    vi.mocked(ask).mockResolvedValue(false);
    vi.mocked(invoke).mockImplementation(async (command) => command === 'validate_backup_archive'
      ? { profileCount: 0, profileNames: [], hasSettings: true, ruleCount: 0, ruleSetCount: 0, appVersion: '0.8.0', exportedAt: 1 }
      : null);
    useConnectionStore.setState({ isConnected: false, connectionStatus: 'disconnected' });
    const { container } = render(<SettingsView />);
    fireEvent.click(screen.getByText('Diagnostics'));
    const archive = { metadata: { schemaVersion: 1, appVersion: '0.8.0', exportedAt: 1, platform: 'windows' }, settings: { proxyMode: 'tun' }, profileConfigs: {} };
    const file = new File([JSON.stringify(archive)], 'backup.json');
    Object.defineProperty(file, 'text', { value: async () => JSON.stringify(archive) });
    fireEvent.change(container.querySelector('input[type="file"]')!, { target: { files: [file] } });
    await waitFor(() => expect(screen.getByText('Backup from X-Link 0.8.0')).toBeTruthy());
    fireEvent.click(screen.getByText('Restore selected sections'));
    await waitFor(() => expect(ask).toHaveBeenCalled());
    expect(vi.mocked(invoke)).not.toHaveBeenCalledWith('import_config_backup', expect.anything());
  });

  it('rejects invalid backup inspection without invoking restore', async () => {
    const { container } = render(<SettingsView />);
    fireEvent.click(screen.getByText('Diagnostics'));
    const fileInput = container.querySelector('input[type="file"]') as HTMLInputElement;
    const file = new File(['not-json'], 'backup.json');
    Object.defineProperty(file, 'text', { value: async () => 'not-json' });
    fireEvent.change(fileInput, { target: { files: [file] } });
    await waitFor(() => expect(screen.getByRole('alert').textContent).toContain('Backup validation failed'));
    expect(vi.mocked(invoke)).not.toHaveBeenCalledWith('import_config_backup', expect.anything());
  });
});
