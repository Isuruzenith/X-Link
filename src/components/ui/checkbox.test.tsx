// @vitest-environment jsdom
import { fireEvent, render } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { Checkbox } from './checkbox';

describe('Material checkbox adapter', () => {
  it('reads checked from the Material checkbox change event', () => {
    const onCheckedChange = vi.fn();
    const { container } = render(<Checkbox checked={false} onCheckedChange={onCheckedChange} aria-label="Allow Insecure" />);
    const control = container.querySelector('md-checkbox')!;
    Object.defineProperty(control, 'checked', { value: true, configurable: true });
    fireEvent.change(control);
    expect(onCheckedChange).toHaveBeenCalledWith(true);
    expect(control.getAttribute('aria-label')).toBe('Allow Insecure');
  });
});
