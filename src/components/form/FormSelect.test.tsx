// @vitest-environment jsdom
import { render, screen, fireEvent } from '@testing-library/react';
import { vi } from 'vitest';
import { describe, expect, it } from 'vitest';
import { FormSelect } from './FormSelect';

describe('FormSelect', () => {
  it('gives a labeled settings select an accessible name', () => {
    render(<FormSelect label="Final outbound" value="proxy" onChange={() => {}} options={[{ value: 'proxy', label: 'Proxy' }]} />);
    expect(screen.getByRole('combobox', { name: 'Final outbound' })).toBeTruthy();
  });

  it('allows choosing a real empty option without clearing the select', () => {
    const onChange = vi.fn();
    render(<FormSelect label="Flow" value="xtls-rprx-vision" onChange={onChange} options={[
      { value: '', label: 'None' }, { value: 'xtls-rprx-vision', label: 'Vision' },
    ]} />);
    fireEvent.click(screen.getByRole('combobox', { name: 'Flow' }));
    fireEvent.click(screen.getByRole('option', { name: 'None' }));
    expect(onChange).toHaveBeenCalledWith('');
  });
});
