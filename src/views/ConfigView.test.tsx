// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { readText } from '@tauri-apps/plugin-clipboard-manager';
import { useProfileStore } from '../stores/profileStore';
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
  });
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
});
