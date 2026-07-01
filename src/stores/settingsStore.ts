import { create } from 'zustand';
import { storeHelper, type Settings } from '../utils/store';
import { invoke } from '@tauri-apps/api/core';

export type ThemeMode = 'dark' | 'light' | 'system';

interface SettingsState {
  settings: Settings;
  conflictingPorts: number[];
  isElevated: boolean;
  singboxVersion: string;
  appVersion: string;
  theme: ThemeMode;
  
  // Actions
  initSettings: () => Promise<void>;
  updateSettings: (updates: Partial<Settings>) => Promise<void>;
  setConflictingPorts: (ports: number[]) => void;
  checkElevated: () => Promise<boolean>;
  fetchVersions: () => Promise<void>;
  setTheme: (theme: ThemeMode) => void;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: {
    proxyMode: 'tun',
    closeToTray: true,
    autostart: false,
    httpPort: 7890,
    socksPort: 7891,
    mixedPort: 7892,
    wifiSharing: false,
    tunAutoRoute: true,
    tunAutoRedirect: false,
    tunStrictRoute: true,
    tunStack: 'gvisor',
    tunMtu: 1500,
    tunEndpointIndependentNat: false,
    sniffEnabled: true,
    sniffHttp: true,
    sniffTls: true,
    sniffQuic: true,
    sniffOverrideDestination: false,
    apiEnabled: false,
    apiPort: 9090,
    apiSecret: '',
    apiCors: true,
    primaryDns: 'https://1.1.1.1/dns-query',
    fallbackDns: 'https://8.8.8.8/dns-query',
    directDns: '223.5.5.5',
    dnsStrategy: 'prefer_ipv4',
    dnsMode: 'fakeip',
    fakeipRange: '198.18.0.0/15',
    fakeipFilter: 'geosite:private',
    dnsLeakProtection: true,
    dnsCaching: true,
    finalOutbound: 'proxy',
    bypassLan: true,
    dnsAddress: '',
  },
  conflictingPorts: [],
  isElevated: false,
  singboxVersion: 'Unknown',
  appVersion: '0.1.0',
  theme: 'dark',

  initSettings: async () => {
    const savedSettings = await storeHelper.getSettings();
    set({ settings: savedSettings });

    // Apply saved theme
    const savedTheme = (savedSettings as any).theme as ThemeMode | undefined;
    if (savedTheme) {
      get().setTheme(savedTheme);
    }
  },

  updateSettings: async (updates) => {
    const current = get().settings;
    const next = { ...current, ...updates };
    set({ settings: next });
    await storeHelper.saveSettings(updates);

    // Apply auto-start and admin elevation configurations via Tauri commands
    if (updates.proxyMode !== undefined) {
      if (updates.proxyMode === 'tun' && get().isElevated) {
        try { await invoke('set_runas_admin', { enabled: true }); } catch {}
      } else if (updates.proxyMode === 'system') {
        try { await invoke('set_runas_admin', { enabled: false }); } catch {}
      }
    }
    if (updates.autostart !== undefined) {
      try { await invoke('set_autostart', { enabled: updates.autostart }); } catch {}
    }
  },

  setConflictingPorts: (ports) => set({ conflictingPorts: ports }),

  checkElevated: async () => {
    try {
      const elevated = await invoke<boolean>('check_tun_support');
      set({ isElevated: elevated });
      return elevated;
    } catch {
      set({ isElevated: false });
      return false;
    }
  },

  fetchVersions: async () => {
    try {
      const version = await invoke<string>('get_app_version');
      set({ appVersion: version });
    } catch {}

    try {
      const ver = await invoke<string>('get_singbox_version');
      set({ singboxVersion: ver });
    } catch {
      set({ singboxVersion: 'Unknown' });
    }
  },

  setTheme: (theme) => {
    set({ theme });
    storeHelper.saveSettings({ theme } as any);

    const resolved = theme === 'system'
      ? (window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark')
      : theme;

    document.documentElement.setAttribute('data-theme', resolved);
  },
}));
