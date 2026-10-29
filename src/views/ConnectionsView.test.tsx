// @vitest-environment jsdom
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { useConnectionStore } from '../stores/connectionStore';
import { ConnectionsView } from './ConnectionsView';

describe('Connections empty state', () => {
  it('offers navigation to the Dashboard when disconnected', () => {
    useConnectionStore.setState({ isConnected: false });
    const onNavigateToTab = vi.fn();
    render(<ConnectionsView onNavigateToTab={onNavigateToTab} />);
    expect(screen.getByText('Proxy service is disconnected.')).toBeTruthy();
    expect(document.querySelector('md-outlined-text-field')?.hasAttribute('disabled')).toBe(true);
    expect(document.querySelectorAll('.material-segment[disabled]')).toHaveLength(3);
    fireEvent.click(screen.getByText('Go to Dashboard'));
    expect(onNavigateToTab).toHaveBeenCalledExactlyOnceWith('dashboard');
  });
});
