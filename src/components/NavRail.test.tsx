// @vitest-environment jsdom
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { NavRail } from './NavRail';

beforeEach(() => {
  window.matchMedia = vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  }));
});

describe('NavRail', () => {
  it('keeps collapsed navigation destinations named and navigable', () => {
    const onTabChange = vi.fn();
    render(<NavRail activeTab="dashboard" onTabChange={onTabChange} />);

    const dashboard = screen.getByRole('button', { name: 'Dashboard' });
    expect(dashboard.getAttribute('aria-current')).toBe('page');
    expect(dashboard.getAttribute('title')).toBe('Dashboard');
    fireEvent.click(screen.getByRole('button', { name: 'Profiles' }));
    expect(onTabChange).toHaveBeenCalledWith('profiles');
  });

  it('provides dedicated class names and titles for intermediate rail text elements', () => {
    const { container } = render(<NavRail activeTab="dashboard" onTabChange={vi.fn()} />);

    expect(container.querySelector('.nav-brand-text')).toBeTruthy();
    expect(container.querySelector('.nav-brand-text strong')?.textContent).toBe('X-Link');

    const session = container.querySelector('.nav-session');
    expect(session).toBeTruthy();
    expect(session?.getAttribute('title')).toContain('Proxy session');
    expect(session?.querySelector('.nav-session-text')).toBeTruthy();

    const labels = container.querySelectorAll('.nav-label');
    expect(labels.length).toBe(6);
  });
});
