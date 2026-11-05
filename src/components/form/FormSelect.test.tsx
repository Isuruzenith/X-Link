// @vitest-environment jsdom
import { render, fireEvent } from '@testing-library/react';
import { vi } from 'vitest';
import { describe, expect, it } from 'vitest';
import { FormSelect } from './FormSelect';

describe('FormSelect', () => {
  it('gives a labeled settings select an accessible name and options', () => {
    const { container } = render(
      <FormSelect label="Final outbound" value="proxy" onChange={() => {}} options={[{ value: 'proxy', label: 'Proxy' }]} />
    );
    const select = container.querySelector('md-outlined-select')!;
    expect(select).toBeTruthy();
    expect(select.getAttribute('label')).toBe('Final outbound');
    expect(select.getAttribute('aria-label')).toBe('Final outbound');
    const option = select.querySelector('md-select-option')!;
    expect(option.getAttribute('value')).toBe('proxy');
  });

  it('forwards accessible validation state to the Material select', () => {
    const { container } = render(<FormSelect label="Rule value" value="" onChange={() => {}} options={[{ value: '', label: 'Select set' }]} aria-invalid={true} aria-describedby="rule-error" />);
    const select = container.querySelector('md-outlined-select')!;
    expect(select.getAttribute('aria-invalid')).toBe('true');
    expect(select.getAttribute('aria-describedby')).toBe('rule-error');
    expect(select.hasAttribute('error')).toBe(true);
  });

  it('allows choosing a real empty option without clearing the select', () => {
    const onChange = vi.fn();
    const { container } = render(
      <FormSelect label="Flow" value="xtls-rprx-vision" onChange={onChange} options={[
        { value: '', label: 'None' }, { value: 'xtls-rprx-vision', label: 'Vision' },
      ]} />
    );
    const select = container.querySelector('md-outlined-select')!;
    Object.defineProperty(select, 'value', { value: '', configurable: true });
    fireEvent.change(select);
    expect(onChange).toHaveBeenCalledWith('');
  });
});

