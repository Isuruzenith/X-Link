import { Store } from '@tauri-apps/plugin-store';

export interface Profile {
  id: string;
  name: string;
  type: 'url' | 'file' | 'manual';
  subscriptionUrl?: string;
  configPath: string;
  lastUpdated: number;
  nodeCount: number;
}

export interface Settings {
  proxyMode: 'system' | 'tun';
  closeToTray: boolean;
  autostart: boolean;
  httpPort: number;
  socksPort: number;
  mixedPort: number;
  dnsAddress: string;
  sniHost: string;
  wifiSharing: boolean;
}

const DEFAULT_SETTINGS: Settings = {
  proxyMode: 'system',
  closeToTray: true,
  autostart: false,
  httpPort: 7890,
  socksPort: 7891,
  mixedPort: 7892,
  dnsAddress: '',
  sniHost: 'aka.ms',
  wifiSharing: false,
};

let settingsStore: Store | null = null;
let profilesStore: Store | null = null;

async function getSettingsStore(): Promise<Store> {
  if (!settingsStore) {
    settingsStore = await Store.load('settings.json');
  }
  return settingsStore;
}

async function getProfilesStore(): Promise<Store> {
  if (!profilesStore) {
    profilesStore = await Store.load('profiles.json');
  }
  return profilesStore;
}

export const storeHelper = {
  // ── SETTINGS STORE ──
  async getSettings(): Promise<Settings> {
    try {
      const store = await getSettingsStore();
      const mode = await store.get<Settings['proxyMode']>('proxyMode') ?? DEFAULT_SETTINGS.proxyMode;
      const closeToTray = await store.get<boolean>('closeToTray') ?? DEFAULT_SETTINGS.closeToTray;
      const autostart = await store.get<boolean>('autostart') ?? DEFAULT_SETTINGS.autostart;
      const httpPort = await store.get<number>('httpPort') ?? DEFAULT_SETTINGS.httpPort;
      const socksPort = await store.get<number>('socksPort') ?? DEFAULT_SETTINGS.socksPort;
      const mixedPort = await store.get<number>('mixedPort') ?? DEFAULT_SETTINGS.mixedPort;
      const dnsAddress = await store.get<string>('dnsAddress') ?? DEFAULT_SETTINGS.dnsAddress;
      const sniHost = await store.get<string>('sniHost') ?? DEFAULT_SETTINGS.sniHost;
      const wifiSharing = await store.get<boolean>('wifiSharing') ?? DEFAULT_SETTINGS.wifiSharing;

      return { proxyMode: mode, closeToTray, autostart, httpPort, socksPort, mixedPort, dnsAddress, sniHost, wifiSharing };
    } catch (e) {
      console.error('Failed to get settings from store, falling back to defaults:', e);
      return { ...DEFAULT_SETTINGS };
    }
  },

  async saveSettings(settings: Partial<Settings>): Promise<void> {
    try {
      const store = await getSettingsStore();
      if (settings.proxyMode !== undefined) await store.set('proxyMode', settings.proxyMode);
      if (settings.closeToTray !== undefined) await store.set('closeToTray', settings.closeToTray);
      if (settings.autostart !== undefined) await store.set('autostart', settings.autostart);
      if (settings.httpPort !== undefined) await store.set('httpPort', settings.httpPort);
      if (settings.socksPort !== undefined) await store.set('socksPort', settings.socksPort);
      if (settings.mixedPort !== undefined) await store.set('mixedPort', settings.mixedPort);
      if (settings.dnsAddress !== undefined) await store.set('dnsAddress', settings.dnsAddress);
      if (settings.sniHost !== undefined) await store.set('sniHost', settings.sniHost);
      if (settings.wifiSharing !== undefined) await store.set('wifiSharing', settings.wifiSharing);

      await store.save();
    } catch (e) {
      console.error('Failed to save settings to store:', e);
    }
  },

  // ── PROFILES STORE ──
  async getProfiles(): Promise<Profile[]> {
    try {
      const store = await getProfilesStore();
      const list = await store.get<Profile[]>('profiles');
      return list || [];
    } catch (e) {
      console.error('Failed to get profiles from store:', e);
      return [];
    }
  },

  async saveProfiles(profiles: Profile[]): Promise<void> {
    try {
      const store = await getProfilesStore();
      await store.set('profiles', profiles);
      await store.save();
    } catch (e) {
      console.error('Failed to save profiles to store:', e);
    }
  },

  async addProfile(profile: Profile): Promise<void> {
    const list = await this.getProfiles();
    // Prevent duplicate IDs
    const filtered = list.filter((p) => p.id !== profile.id);
    filtered.push(profile);
    await this.saveProfiles(filtered);
  },

  async removeProfile(profileId: string): Promise<void> {
    const list = await this.getProfiles();
    const filtered = list.filter((p) => p.id !== profileId);
    await this.saveProfiles(filtered);
  },
};
