// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { useConnectionStore } from '../stores/connectionStore';
import { ConnectionsView } from './ConnectionsView';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ ask: vi.fn() }));

afterEach(cleanup);

describe('Connections empty state', () => {
  it('offers navigation to the Dashboard when disconnected', () => {
    useConnectionStore.setState({ isConnected: false });
    const onNavigateToTab = vi.fn();
    render(<ConnectionsView onNavigateToTab={onNavigateToTab} />);
    expect(screen.getByText('Proxy service is disconnected.')).toBeTruthy();
    expect(document.querySelector('.max-w-md')).toBeTruthy();
    expect(document.querySelector('md-outlined-text-field')?.hasAttribute('disabled')).toBe(true);
    expect(document.querySelectorAll('.material-segment[disabled]')).toHaveLength(3);
    fireEvent.click(screen.getByText('Go to Dashboard'));
    expect(onNavigateToTab).toHaveBeenCalledExactlyOnceWith('dashboard');
  });

  it('shows connection details after choosing a destination', async () => {
    vi.mocked(invoke).mockResolvedValue({ connections: [{
      id: 'conn-1',
      metadata: { host: 'example.com', destinationIP: '1.2.3.4', destinationPort: '443', sourceIP: '127.0.0.1', sourcePort: '5050', type: 'http', network: 'tcp' },
      upload: 100, download: 200, start: new Date().toISOString(), chains: ['proxy'], rule: 'DOMAIN',
    }] });
    useConnectionStore.setState({ isConnected: true });
    render(<ConnectionsView />);
    const destination = await screen.findByRole('button', { name: /example.com/ });
    fireEvent.click(destination);
    expect(screen.getByRole('region', { name: 'Selected connection details' })).toBeTruthy();
    expect(screen.getByText('127.0.0.1:5050')).toBeTruthy();
    fireEvent.click(screen.getByLabelText('Close connection details'));
    expect(screen.queryByRole('region', { name: 'Selected connection details' })).toBeNull();
  });
});
