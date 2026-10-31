// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { readText } from '@tauri-apps/plugin-clipboard-manager';
import { useProfileStore } from '../stores/profileStore';
import { useSettingsStore } from '../stores/settingsStore';
import { useConnectionStore } from '../stores/connectionStore';
import { ConfigView } from './ConfigView';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ ask: vi.fn(), open: vi.fn() }));
vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({ readText: vi.fn() }));
vi.mock('../utils/store', () => ({
  storeHelper: { saveProfiles: vi.fn(), setActiveProfileId: vi.fn() },
}));

afterEach(cleanup);

beforeEach(() => {
  vi.clearAllMocks();
  useProfileStore.setState({
    profiles: [], activeProfileId: null, selectedProfileId: null, nodes: [], selectedNodeTag: null,
    importName: '', importContent: '', importError: null, importSuccess: false, isImporting: false,
    latencyResults: {}, isTestingLatency: false, testingNodeTag: null,
  });
  useConnectionStore.setState({ isConnected: false });
  useSettingsStore.setState({ settings: { ...useSettingsStore.getState().settings, apiEnabled: false } });
});

describe('Profiles import', () => {
  it('opens the import form from the empty state and validates pasted URLs', () => {
    render(<ConfigView />);
    expect(screen.queryByRole('heading', { name: 'Add a profile' })).toBeNull();
    fireEvent.click(screen.getByText('Add your first profile'));
    expect(screen.getByRole('heading', { name: 'Add a profile' })).toBeTruthy();
    act(() => useProfileStore.getState().setImportContent('https:/broken'));
    expect(screen.getByRole('alert').textContent).toContain('Enter a valid subscription URL');
    expect(screen.getByText('Import profile').closest('button, md-filled-button')?.hasAttribute('disabled')).toBe(true);
  });

  it('validates content pasted from the clipboard', async () => {
    vi.mocked(readText).mockResolvedValue('https:/broken');
    render(<ConfigView />);
    fireEvent.click(screen.getByText('Add your first profile'));
    fireEvent.click(screen.getByText('Paste clipboard'));
    await waitFor(() => expect(screen.getByRole('alert').textContent).toContain('Enter a valid subscription URL'));
    expect(screen.getByText('Import profile').closest('button, md-filled-button')?.hasAttribute('disabled')).toBe(true);
  });

  it('imports valid content and returns to the saved profiles view', async () => {
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'import_config') return { id: 'new', name: 'My Config', importedAt: 1, nodeCount: 0 };
      if (command === 'get_profile_outbounds') return [];
      return null;
    });
    render(<ConfigView />);
    fireEvent.click(screen.getByText('Add your first profile'));
    act(() => useProfileStore.getState().setImportContent('https://example.com/config'));
    fireEvent.click(screen.getByText('Import profile'));
    await waitFor(() => expect(screen.queryByRole('heading', { name: 'Add a profile' })).toBeNull());
    expect(vi.mocked(invoke)).toHaveBeenCalledWith('import_config', expect.objectContaining({ content: 'https://example.com/config' }));
    expect(screen.getByRole('button', { name: /My Config/ })).toBeTruthy();
  });

  it('creates an empty manual profile via the empty profile button', async () => {
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'create_empty_profile') return { id: 'manual-1', name: 'Manual Profile', type: 'manual', importedAt: 1, lastUpdated: 1, nodeCount: 0 };
      if (command === 'get_profile_outbounds') return [];
      return null;
    });
    render(<ConfigView />);
    fireEvent.click(screen.getByText('Add your first profile'));
    fireEvent.click(screen.getByText('Empty profile'));
    await waitFor(() => expect(screen.queryByRole('heading', { name: 'Add a profile' })).toBeNull());
    expect(vi.mocked(invoke)).toHaveBeenCalledWith('create_empty_profile', { name: 'Manual Profile' });
  });

  it('shows a direct TCP control and labels its result', async () => {
    useProfileStore.setState({
      profiles: [{ id: 'p1', name: 'Profile 1', type: 'manual', importedAt: 1, lastUpdated: 1, nodeCount: 1, selectedNodeTag: 'node-1' }],
      activeProfileId: 'p1', selectedProfileId: 'p1',
      nodes: [{ tag: 'node-1', type: 'vless', server: '192.0.2.1', server_port: 443 }],
    });
    vi.mocked(invoke).mockResolvedValue(18 as never);
    const { container } = render(<ConfigView />);
    expect(container.querySelector('md-icon-button[aria-label="Test proxy URL for node-1"]')).toBeNull();
    const control = container.querySelector('md-icon-button[aria-label="Test direct TCP for node-1"]') as HTMLElement;
    fireEvent.click(control);
    await waitFor(() => expect(screen.getByText('TCP: 18 ms')).toBeTruthy());
    expect(vi.mocked(invoke)).toHaveBeenCalledWith('test_node_latency', { server: '192.0.2.1', port: 443 });
  });

  it('shows proxy URL test only for the active profile with an enabled API', () => {
    useProfileStore.setState({
      profiles: [{ id: 'p1', name: 'Profile 1', type: 'manual', importedAt: 1, lastUpdated: 1, nodeCount: 1, selectedNodeTag: 'node-1' }],
      activeProfileId: 'p1', selectedProfileId: 'p1',
      nodes: [{ tag: 'node-1', type: 'vless', server: '192.0.2.1', server_port: 443 }],
    });
    useConnectionStore.setState({ isConnected: true });
    useSettingsStore.setState({ settings: { ...useSettingsStore.getState().settings, apiEnabled: true } });
    const { container } = render(<ConfigView />);
    expect(container.querySelector('md-icon-button[aria-label="Test proxy URL for node-1"]')).toBeTruthy();
  });

  it('deletes a node when confirmed', async () => {
    const { ask } = await import('@tauri-apps/plugin-dialog');
    vi.mocked(ask).mockResolvedValue(true);
    useProfileStore.setState({
      profiles: [{ id: 'p1', name: 'Profile 1', type: 'manual', importedAt: 1, lastUpdated: 1, nodeCount: 1, selectedNodeTag: 'node-1' }],
      activeProfileId: 'p1',
      selectedProfileId: 'p1',
      nodes: [{ tag: 'node-1', type: 'vmess', server: '1.2.3.4', server_port: 443 }],
    });
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'delete_node') return undefined;
      if (command === 'get_profile_outbounds') return [];
      return null;
    });

    const { container } = render(<ConfigView />);
    const deleteBtn = container.querySelector('md-icon-button[aria-label="Delete node-1"]') as HTMLElement;
    expect(deleteBtn).toBeTruthy();
    fireEvent.click(deleteBtn);
    await waitFor(() => {
      expect(ask).toHaveBeenCalledWith(expect.stringContaining('Delete node "node-1"?'), expect.anything());
      expect(vi.mocked(invoke)).toHaveBeenCalledWith('delete_node', { tag: 'node-1' });
    });
  });
});
