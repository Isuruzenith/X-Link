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
  });
  useConnectionStore.setState({ connectionStatus: 'connected', isConnected: true });
  useSettingsStore.setState({
    settings: { ...useSettingsStore.getState().settings, geoLookupEnabled: false },
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
