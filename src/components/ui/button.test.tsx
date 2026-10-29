// @vitest-environment jsdom
import { createElement } from 'react';
import { render } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { Button } from './button';
import { Icon } from './icon';

function OtherChild({ name }: { name: string }) {
  return <span>{name}</span>;
}

describe('Material button adapter', () => {
  it('preserves submit behavior for the profile import form', () => {
    const submit = vi.fn((event: React.FormEvent<HTMLFormElement>) => event.preventDefault());
    const { container } = render(<form onSubmit={submit}><Button type="submit">Import</Button></form>);
    const action = container.querySelector('md-filled-button')!;
    expect(action.getAttribute('type')).toBe('submit');
    // jsdom does not implement form-associated custom element activation.
    // The browser-level submit path must be checked in the running app.
    container.querySelector('form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    expect(submit).toHaveBeenCalledOnce();
  });

  it('serialises boolean ARIA state as true/false tokens', () => {
    const { container, rerender } = render(<Button aria-expanded={false}>Toggle</Button>);
    expect(container.querySelector('md-filled-button')!.getAttribute('aria-expanded')).toBe('false');
    rerender(<Button aria-expanded>Toggle</Button>);
    expect(container.querySelector('md-filled-button')!.getAttribute('aria-expanded')).toBe('true');
  });

  it('names icon-only actions and keeps other buttons non-submitting', () => {
    const { container } = render(<Button variant="ghost" size="icon" title="Delete profile">×</Button>);
    const action = container.querySelector('md-icon-button')!;
    expect(action.getAttribute('aria-label')).toBe('Delete profile');
    expect(action.getAttribute('type')).toBe('button');
  });

  it('slots Icon and md-icon children without slotting other elements', () => {
    const { container } = render(
      <Button variant="outline">
        <Icon name="upload_file" />
        {createElement('md-icon', null, 'paste')}
        <OtherChild name="file" />
        Choose file
      </Button>,
    );
    expect(container.querySelectorAll('md-icon[slot="icon"]')).toHaveLength(2);
    expect(container.querySelector('span')?.hasAttribute('slot')).toBe(false);
  });

  it('slots elements explicitly marked as button icons', () => {
    const { container } = render(<Button><span data-icon="true">Icon</span> Paste clipboard</Button>);
    expect(container.querySelector('span')?.getAttribute('slot')).toBe('icon');
  });

  it('preserves an explicitly chosen slot', () => {
    const { container } = render(<Button><Icon name="arrow_forward" slot="trailing-icon" /> Next</Button>);
    expect(container.querySelector('md-icon')?.getAttribute('slot')).toBe('trailing-icon');
  });

  it('keeps the default slot on icon-only buttons', () => {
    const { container } = render(<Button size="icon" title="Close"><Icon name="close" /></Button>);
    expect(container.querySelector('md-icon')?.hasAttribute('slot')).toBe(false);
  });
});
