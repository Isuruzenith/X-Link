// @vitest-environment jsdom
import { fireEvent, render } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { Switch } from './switch';

describe('Material switch adapter', () => {
  it('reads selected from the Material switch input event', () => {
    const onCheckedChange = vi.fn();
    const { container } = render(<Switch checked={false} onCheckedChange={onCheckedChange} aria-label="Enable TUN" />);
    const control = container.querySelector('md-switch')!;
    Object.defineProperty(control, 'selected', { value: true, configurable: true });
    fireEvent.input(control);
    expect(onCheckedChange).toHaveBeenCalledWith(true);
    expect(control.getAttribute('aria-label')).toBe('Enable TUN');
  });
});
