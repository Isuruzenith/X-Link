// @vitest-environment jsdom
import { render, fireEvent } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { FormInput } from './FormInput';

describe('FormInput', () => {
  it('passes Material field input values to the form callback', () => {
    const onChange = vi.fn();
    const { container } = render(<FormInput value="" label="Profile name" onChange={onChange} />);
    const field = container.querySelector('md-outlined-text-field')!;
    Object.defineProperty(field, 'value', { value: 'Updated profile', configurable: true });
    fireEvent.input(field);
    expect(onChange).toHaveBeenCalledWith('Updated profile');
    expect(field.getAttribute('aria-label')).toBe('Profile name');
  });

  it('forwards accessible Material validation state', () => {
    const { container } = render(<FormInput value="" label="Rule value" onChange={() => {}} aria-invalid={true} aria-describedby="rule-error" />);
    const field = container.querySelector('md-outlined-text-field')!;
    expect(field.getAttribute('aria-invalid')).toBe('true');
    expect(field.getAttribute('aria-describedby')).toBe('rule-error');
    expect(field.hasAttribute('error')).toBe(true);
  });

  it('forwards native change events from a Material field', () => {
    const onFieldChange = vi.fn();
    const { container } = render(<FormInput value="2080" label="Port" onChange={() => {}} onFieldChange={onFieldChange} />);
    const field = container.querySelector('md-outlined-text-field')!;
    Object.defineProperty(field, 'value', { configurable: true, value: '3000' });
    fireEvent.change(field);
    expect(onFieldChange).toHaveBeenCalledWith('3000');
  });

  it('forwards textarea rows and its accessible name for pasted configurations', () => {
    const { container } = render(<FormInput label="Link or configuration" value="" onChange={() => {}} type="textarea" rows={3} />);
    const field = container.querySelector('md-outlined-text-field')!;
    expect(field.getAttribute('type')).toBe('textarea');
    expect(field.getAttribute('rows')).toBe('3');
    expect(field.getAttribute('aria-label')).toBe('Link or configuration');
  });
});
