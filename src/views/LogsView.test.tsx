// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useLogStore } from '../stores/logStore';
import { LogsView } from './LogsView';

describe('LogsView', () => {
  afterEach(cleanup);

  beforeEach(() => {
    global.ResizeObserver = class ResizeObserver {
      observe() {}
      unobserve() {}
      disconnect() {}
    } as unknown as typeof ResizeObserver;
    useLogStore.setState({ logs: [], autoScroll: true });
  });

  it('renders log filter controls and toolbar correctly', () => {
    useLogStore.getState().pushLog('info', 'Sample info message');
    render(<LogsView />);

    expect(screen.getByText('Follow proxy activity and system events as they happen')).toBeTruthy();
    expect(screen.getByText('Copy All')).toBeTruthy();
    expect(screen.getByText('Clear')).toBeTruthy();
    expect(screen.getByText('Autoscroll')).toBeTruthy();
    expect(screen.getByRole('region', { name: 'Proxy activity logs' })).toBeTruthy();
  });

  it('turns autoscroll off when scrolling away and on when returning to the bottom', () => {
    useLogStore.getState().pushLog('info', 'Sample info message');
    render(<LogsView />);
    const list = screen.getByRole('region', { name: 'Proxy activity logs' }).firstElementChild as HTMLElement;
    Object.defineProperties(list, {
      scrollHeight: { configurable: true, value: 600 },
      clientHeight: { configurable: true, value: 100 },
      scrollTop: { configurable: true, writable: true, value: 0 },
    });
    fireEvent.scroll(list);
    expect(useLogStore.getState().autoScroll).toBe(false);
    list.scrollTop = 500;
    fireEvent.scroll(list);
    expect(useLogStore.getState().autoScroll).toBe(true);
  });

  it('reports a clipboard failure rather than claiming a successful copy', async () => {
    const clipboard = { writeText: vi.fn().mockRejectedValue(new Error('denied')) };
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: clipboard });
    useLogStore.getState().pushLog('info', 'Sample info message');
    render(<LogsView />);
    fireEvent.click(screen.getByText('Copy All'));
    const { useToastStore } = await import('../stores/toastStore');
    await vi.waitFor(() => expect(useToastStore.getState().toasts.some((toast) => toast.title === 'Copy Failed')).toBe(true));
  });
});
