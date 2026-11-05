// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useRoutingStore } from '../stores/routingStore';
import { RoutingView } from './RoutingView';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

describe('RoutingView rules table layout', () => {
  afterEach(cleanup);

  beforeEach(() => {
    useRoutingStore.setState({
      showAddRule: false,
      showAddRuleSet: false,
      editingRule: null,
      ruleForm: { type: 'geoip', value: '', outbound: 'direct', invert: false, notes: '' },
      ruleSetForm: { tag: '', type: 'remote', format: 'binary', url: '', updateInterval: '1d' },
      routingRules: [
        { id: '1', type: 'domain', value: 'example.com', outbound: 'direct', enabled: true, invert: false, notes: 'Direct test' },
        { id: '2', type: 'ip_cidr', value: '1.1.1.1/32', outbound: 'proxy', enabled: true, invert: false, notes: '' },
      ],
      ruleSets: [],
    });
  });

  it('renders routing table with dedicated actions header and no column clipping', () => {
    const { container } = render(<RoutingView />);

    const scrollContainer = container.querySelector('.routing-table-scroll');
    expect(scrollContainer).toBeTruthy();

    const header = container.querySelector('.rules-table-header');
    expect(header).toBeTruthy();
    expect(screen.getByText('Actions')).toBeTruthy();
    expect(screen.getByText('example.com')).toBeTruthy();
    expect(screen.getByLabelText('Enable rule example.com')).toBeTruthy();
  });

  it('explains why a rule without a value cannot be added', () => {
    render(<RoutingView />);
    fireEvent.click(screen.getByText('Add Rule'));
    fireEvent.click(screen.getByText('Add', { selector: 'md-filled-button' }));
    expect(screen.getByRole('alert').textContent).toBe('Enter a rule value.');
    expect(useRoutingStore.getState().routingRules).toHaveLength(2);
  });

  it('requires a tag and a remote URL before saving a remote rule set', () => {
    render(<RoutingView />);
    fireEvent.click(screen.getByText('Add Rule Set'));
    fireEvent.click(screen.getByText('Add', { selector: 'md-filled-button' }));
    expect(screen.getByRole('alert').textContent).toBe('Enter a tag for the rule set.');
    act(() => useRoutingStore.getState().setRuleSetForm({ tag: 'geoip-cn' }));
    fireEvent.click(screen.getByText('Add', { selector: 'md-filled-button' }));
    expect(screen.getByRole('alert').textContent).toBe('Enter a remote URL for the rule set.');
    expect(useRoutingStore.getState().ruleSets).toHaveLength(0);
  });

  it('guards remote rule sets at the store boundary', async () => {
    useRoutingStore.getState().setRuleSetForm({ tag: 'geoip-cn', type: 'remote', url: '  ' });
    await useRoutingStore.getState().saveRuleSet();
    expect(useRoutingStore.getState().ruleSets).toHaveLength(0);
  });
});
