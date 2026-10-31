import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { storeHelper, type Profile } from '../utils/store';
import { useProfileStore } from './profileStore';
import { useConnectionStore } from './connectionStore';
import { useSettingsStore } from './settingsStore';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({ readText: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
vi.mock('../utils/store', () => ({
  storeHelper: {
    deleteProfile: vi.fn(),
    saveProfiles: vi.fn(),
    reloadProfiles: vi.fn(),
    setActiveProfileId: vi.fn(),
  },
}));

const profile = (id: string): Profile => ({
  id, name: id, type: 'manual', importedAt: 1, lastUpdated: 1,
  nodeCount: 1, selectedNodeTag: 'old',
});
const native = vi.mocked(invoke);

beforeEach(() => {
  vi.clearAllMocks();
  useProfileStore.setState({
    profiles: [profile('first'), profile('second')], activeProfileId: 'first',
    selectedProfileId: 'first', nodes: [], selectedNodeTag: 'old', nodeGeoCache: {},
    latencyResults: {}, isTestingLatency: false, testingNodeTag: null,
  });
  useConnectionStore.setState({ connectionStatus: 'connected', isConnected: true });
  useSettingsStore.setState({
    settings: { ...useSettingsStore.getState().settings, apiEnabled: false, geoLookupEnabled: false },
  });
});

describe('profile deletion', () => {
  it('stops and confirms disconnect before changing profile metadata', async () => {
    const calls: string[] = [];
    native.mockImplementation(async (command) => {
      calls.push(command);
      if (command === 'get_proxy_status') return { status: calls.includes('toggle_proxy') ? 'disconnected' : 'connected' };
      if (command === 'get_profile_outbounds') return [{ tag: 'old' }];
      return undefined;
    });
    vi.mocked(storeHelper.deleteProfile).mockImplementation(async () => { calls.push('delete_metadata'); });

    await useProfileStore.getState().deleteProfile('first');

    expect(calls.indexOf('toggle_proxy')).toBeGreaterThan(calls.indexOf('get_proxy_status'));
    expect(calls.indexOf('delete_metadata')).toBeGreaterThan(calls.lastIndexOf('get_proxy_status'));
    expect(useProfileStore.getState().profiles.map((item) => item.id)).toEqual(['second']);
    expect(useProfileStore.getState().activeProfileId).toBe('second');
  });

  it('retains the active profile when stopping fails', async () => {
    native.mockImplementation(async (command) => {
      if (command === 'get_proxy_status') return { status: 'connected' };
      if (command === 'toggle_proxy') throw new Error('stop failed');
      return undefined;
    });

    await useProfileStore.getState().deleteProfile('first');

    expect(storeHelper.deleteProfile).not.toHaveBeenCalled();
    expect(native).not.toHaveBeenCalledWith('delete_profile_config', expect.anything());
    expect(useProfileStore.getState().activeProfileId).toBe('first');
    expect(useConnectionStore.getState().isConnected).toBe(true);
  });

  it('restores metadata when native config deletion fails', async () => {
    native.mockImplementation(async (command) => {
      if (command === 'get_proxy_status') return { status: 'disconnected' };
      if (command === 'delete_profile_config') throw new Error('file delete failed');
      return undefined;
    });

    await useProfileStore.getState().deleteProfile('second');

    expect(storeHelper.deleteProfile).toHaveBeenCalledWith('second');
    expect(storeHelper.saveProfiles).toHaveBeenCalledWith([profile('first'), profile('second')]);
    expect(storeHelper.setActiveProfileId).toHaveBeenCalledWith('first');
    expect(useProfileStore.getState().profiles.map((item) => item.id)).toEqual(['first', 'second']);
  });

  it('allows deleting the final profile after confirmed disconnect', async () => {
    useProfileStore.setState({ profiles: [profile('first')], activeProfileId: 'first', selectedProfileId: 'first' });
    native.mockResolvedValue({ status: 'disconnected' } as never);

    await useProfileStore.getState().deleteProfile('first');

    expect(storeHelper.deleteProfile).toHaveBeenCalledWith('first');
    expect(native).toHaveBeenCalledWith('delete_profile_config', { profileId: 'first' });
    expect(useProfileStore.getState().profiles).toEqual([]);
    expect(useProfileStore.getState().activeProfileId).toBeNull();
  });

  it('does not disconnect to delete a non-active profile', async () => {
    native.mockResolvedValue(undefined);
    await useProfileStore.getState().deleteProfile('second');
    expect(native).not.toHaveBeenCalledWith('toggle_proxy', expect.anything());
    expect(storeHelper.deleteProfile).toHaveBeenCalledWith('second');
  });
});

describe('subscription refresh', () => {
  it('invokes native refresh for an active connected profile and reloads its metadata', async () => {
    const subscription = { ...profile('first'), type: 'subscription' as const, subscriptionUrl: 'https://example.com/sub' };
    useProfileStore.setState({ profiles: [subscription, profile('second')] });
    vi.mocked(storeHelper.reloadProfiles).mockResolvedValue([{ ...subscription, nodeCount: 2 }, profile('second')]);
    native.mockImplementation(async (command) => {
      if (command === 'refresh_subscription') return { name: 'first', nodeCount: 2 };
      if (command === 'get_profile_outbounds') return [{ tag: 'old' }];
      return undefined;
    });

    await useProfileStore.getState().refreshSubscription('first');

    expect(native).toHaveBeenCalledWith('refresh_subscription', { profileId: 'first' });
    expect(storeHelper.reloadProfiles).toHaveBeenCalled();
    expect(useProfileStore.getState().profiles[0].nodeCount).toBe(2);
    expect(useProfileStore.getState().refreshingProfileId).toBeNull();
  });

  it('retains previous metadata when native connected refresh rejects', async () => {
    const subscription = { ...profile('first'), type: 'subscription' as const, subscriptionUrl: 'https://example.com/sub' };
    useProfileStore.setState({ profiles: [subscription] });
    native.mockRejectedValue(new Error('restart failed'));

    await useProfileStore.getState().refreshSubscription('first');

    expect(storeHelper.reloadProfiles).not.toHaveBeenCalled();
    expect(useProfileStore.getState().profiles).toEqual([subscription]);
    expect(useProfileStore.getState().refreshingProfileId).toBeNull();
  });
});

describe('tray node events', () => {
  it('persists a valid tag only for the active profile', async () => {
    native.mockResolvedValue([{ tag: 'old' }, { tag: 'new' }]);
    await useProfileStore.getState().syncNodeFromTray('first', 'new');
    expect(storeHelper.saveProfiles).toHaveBeenCalledWith(expect.arrayContaining([
      expect.objectContaining({ id: 'first', selectedNodeTag: 'new' }),
    ]));
    expect(useProfileStore.getState().selectedNodeTag).toBe('new');

    vi.clearAllMocks();
    await useProfileStore.getState().syncNodeFromTray('second', 'new');
    expect(native).not.toHaveBeenCalled();
  });

  it('ignores a tag missing from the active profile', async () => {
    native.mockResolvedValue([{ tag: 'old' }]);
    await useProfileStore.getState().syncNodeFromTray('first', 'missing');
    expect(storeHelper.saveProfiles).not.toHaveBeenCalled();
    expect(useProfileStore.getState().selectedNodeTag).toBe('old');
  });
});

describe('node geolocation consent', () => {
  it('uses tag inference without invoking the network when disabled', async () => {
    await useProfileStore.getState().fetchNodeGeo('8.8.8.8', 'US-1');
    expect(native).not.toHaveBeenCalledWith('lookup_node_geo', expect.anything());
    expect(useProfileStore.getState().nodeGeoCache['8.8.8.8']).toMatchObject({ countryCode: 'us' });
  });

  it('looks up and caches a node after opt-in', async () => {
    useSettingsStore.setState({ settings: { ...useSettingsStore.getState().settings, geoLookupEnabled: true } });
    native.mockResolvedValue({ countryCode: 'GB', countryName: 'United Kingdom', cityName: '', regionName: '' });
    await useProfileStore.getState().fetchNodeGeo('8.8.4.4', 'node');
    await useProfileStore.getState().fetchNodeGeo('8.8.4.4', 'node');
    expect(native).toHaveBeenCalledTimes(1);
    expect(useProfileStore.getState().nodeGeoCache['8.8.4.4']).toMatchObject({ countryCode: 'gb' });
  });
});

describe('profile auto-update interval', () => {
  it('updates autoUpdateInterval and persists profile list', async () => {
    await useProfileStore.getState().setProfileAutoUpdateInterval('first', 360);

    const first = useProfileStore.getState().profiles.find((p) => p.id === 'first');
    expect(first?.autoUpdateInterval).toBe(360);
    expect(storeHelper.saveProfiles).toHaveBeenCalledWith(
      expect.arrayContaining([
        expect.objectContaining({ id: 'first', autoUpdateInterval: 360 }),
      ])
    );
  });
});

describe('empty profile and node management', () => {
  it('creates an empty manual profile and sets it as active', async () => {
    const emptyMeta = {
      id: 'empty-1',
      name: 'My Manual Profile',
      type: 'manual' as const,
      importedAt: Date.now(),
      lastUpdated: Date.now(),
      nodeCount: 0,
    };
    native.mockImplementation(async (command) => {
      if (command === 'create_empty_profile') return emptyMeta;
      if (command === 'get_profile_outbounds') return [];
      return undefined;
    });
    vi.mocked(storeHelper.reloadProfiles).mockResolvedValue([
      profile('first'),
      profile('second'),
      { ...emptyMeta, selectedNodeTag: null },
    ]);

    await useProfileStore.getState().createEmptyProfile('My Manual Profile');

    expect(native).toHaveBeenCalledWith('create_empty_profile', { name: 'My Manual Profile' });
    expect(storeHelper.setActiveProfileId).toHaveBeenCalledWith('empty-1');
    expect(useProfileStore.getState().activeProfileId).toBe('empty-1');
  });

  it('deletes a node and refreshes outbounds', async () => {
    native.mockImplementation(async (command) => {
      if (command === 'delete_node') return undefined;
      if (command === 'get_profile_outbounds') return [];
      return undefined;
    });

    await useProfileStore.getState().deleteNode('node-to-delete');

    expect(native).toHaveBeenCalledWith('delete_node', { tag: 'node-to-delete' });
    expect(native).toHaveBeenCalledWith('get_profile_outbounds', expect.anything());
  });

  it('tests URL latency when connected with Clash API enabled', async () => {
    useConnectionStore.setState({ isConnected: true, connectionStatus: 'connected' });
    useSettingsStore.setState({ settings: { ...useSettingsStore.getState().settings, apiEnabled: true } });
    native.mockImplementation(async (command) => {
      if (command === 'test_all_nodes_url_latency') {
        return [{ tag: 'node-1', latencyMs: 120, error: null }];
      }
      return undefined;
    });

    await useProfileStore.getState().testAllNodes();

    expect(native).toHaveBeenCalledWith('test_all_nodes_url_latency', { profileId: 'first' });
    expect(useProfileStore.getState().latencyResults['node-1']).toEqual({
      latencyMs: 120,
      error: null,
      mode: 'proxy-url',
    });
  });

  it('falls back to TCP latency if an enabled Clash API request fails', async () => {
    useSettingsStore.setState({ settings: { ...useSettingsStore.getState().settings, apiEnabled: true } });
    native.mockImplementation(async (command) => {
      if (command === 'test_all_nodes_url_latency') throw new Error('controller unavailable');
      if (command === 'test_all_nodes') {
        return [{ tag: 'node-1', latencyMs: 45, error: null }];
      }
      return undefined;
    });

    await useProfileStore.getState().testAllNodes();

    expect(native).toHaveBeenCalledWith('test_all_nodes_url_latency', { profileId: 'first' });
    expect(native).toHaveBeenCalledWith('test_all_nodes', { profileId: 'first' });
    expect(useProfileStore.getState().latencyResults['node-1']).toEqual({
      latencyMs: 45,
      error: null,
      mode: 'tcp',
    });
  });

  it('tests TCP latency when connected with Clash API disabled', async () => {
    useConnectionStore.setState({ isConnected: true, connectionStatus: 'connected' });
    native.mockImplementation(async (command) => {
      if (command === 'test_all_nodes') {
        return [{ tag: 'node-1', latencyMs: 45, error: null }];
      }
      return undefined;
    });

    await useProfileStore.getState().testAllNodes();

    expect(native).not.toHaveBeenCalledWith('test_all_nodes_url_latency', expect.anything());
    expect(native).toHaveBeenCalledWith('test_all_nodes', { profileId: 'first' });
    expect(useProfileStore.getState().latencyResults['node-1']).toEqual({
      latencyMs: 45,
      error: null,
      mode: 'tcp',
    });
  });

  it('tests TCP for an inactive profile even when the active core exposes an API', async () => {
    useProfileStore.setState({ selectedProfileId: 'second' });
    useSettingsStore.setState({ settings: { ...useSettingsStore.getState().settings, apiEnabled: true } });
    native.mockResolvedValue([{ tag: 'node-1', latencyMs: 45, error: null }] as never);

    await useProfileStore.getState().testAllNodes();

    expect(native).not.toHaveBeenCalledWith('test_all_nodes_url_latency', expect.anything());
    expect(native).toHaveBeenCalledWith('test_all_nodes', { profileId: 'second' });
    expect(useProfileStore.getState().latencyResults['node-1'].mode).toBe('tcp');
  });

  it('tests one node via direct TCP without selecting it', async () => {
    const node = { tag: 'one', type: 'vless', server: '192.0.2.1', server_port: 8443 };
    useProfileStore.setState({ nodes: [node] });
    native.mockResolvedValue(35 as never);

    await useProfileStore.getState().testNode(node, 'tcp');

    expect(native).toHaveBeenCalledWith('test_node_latency', { server: node.server, port: 8443 });
    expect(useProfileStore.getState().latencyResults.one).toEqual({ latencyMs: 35, error: null, mode: 'tcp' });
    expect(useProfileStore.getState().selectedNodeTag).toBe('old');
  });

  it('reports per-node proxy URL failures without silently relabeling them as TCP', async () => {
    const node = { tag: 'one', type: 'vless', server: '192.0.2.1', server_port: 8443 };
    useProfileStore.setState({ nodes: [node] });
    useSettingsStore.setState({ settings: { ...useSettingsStore.getState().settings, apiEnabled: true } });
    native.mockRejectedValue(new Error('controller failed'));

    await useProfileStore.getState().testNode(node, 'proxy-url');

    expect(native).toHaveBeenCalledWith('test_proxy_url_latency', { tag: 'one' });
    expect(native).not.toHaveBeenCalledWith('test_node_latency', expect.anything());
    expect(useProfileStore.getState().latencyResults.one).toMatchObject({ latencyMs: null, mode: 'proxy-url' });
  });

  it('falls back to TCP ping when disconnected', async () => {
    useConnectionStore.setState({ isConnected: false, connectionStatus: 'disconnected' });
    native.mockImplementation(async (command) => {
      if (command === 'test_all_nodes') {
        return [{ tag: 'node-2', latencyMs: 45, error: null }];
      }
      return undefined;
    });

    await useProfileStore.getState().testAllNodes();

    expect(native).not.toHaveBeenCalledWith('test_all_nodes_url_latency', expect.anything());
    expect(native).toHaveBeenCalledWith('test_all_nodes', { profileId: 'first' });
    expect(useProfileStore.getState().latencyResults['node-2']).toEqual({
      latencyMs: 45,
      error: null,
      mode: 'tcp',
    });
  });
});
