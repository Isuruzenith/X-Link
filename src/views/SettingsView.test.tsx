// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { useSettingsStore } from '../stores/settingsStore';
import { useConnectionStore } from '../stores/connectionStore';
import { useToastStore } from '../stores/toastStore';
import { SettingsView } from './SettingsView';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

afterEach(cleanup);

beforeEach(() => {
  vi.clearAllMocks();
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
});
