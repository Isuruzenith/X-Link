import { Store } from '@tauri-apps/plugin-store';

// ── LEGACY ACTIVE CONFIG (kept for migration) ───────────────────────────────
export interface ActiveConfig {
  /** Always "active" — fixed slot, not a UUID */
  id: 'active';
  name: string;
  importedAt: number;      // Unix ms timestamp
  nodeCount: number;
  selectedNodeTag: string | null;
}

// ── MULTI-PROFILE SYSTEM ─────────────────────────────────────────────────────
export type ProfileType = 'subscription' | 'manual' | 'file';

export interface Profile {
  id: string;
  name: string;
  type: ProfileType;
  subscriptionUrl?: string;
  importedAt: number;        // Unix ms
  lastUpdated: number;       // Unix ms
  nodeCount: number;
  groupId?: string;
  selectedNodeTag: string | null;
}

// ── ROUTING ───────────────────────────────────────────────────────────────────
export type RoutingRuleType =
  | 'domain' | 'domain_suffix' | 'domain_keyword' | 'domain_regex'
  | 'ip_cidr' | 'geoip' | 'geosite' | 'port' | 'port_range'
  | 'protocol' | 'process_name' | 'rule_set' | 'network' | 'package_name';

export type OutboundAction = 'proxy' | 'direct' | 'block' | 'dns';

export interface RoutingRule {
  id: string;
  type: RoutingRuleType;
  value: string;
  outbound: OutboundAction;
  invert: boolean;
  notes?: string;
  enabled?: boolean;
}

export interface RuleSet {
  id: string;
  tag: string;
  type: 'remote' | 'local';
  format: 'binary' | 'source';
  url?: string;
  filePath?: string;
  updateInterval: string; // e.g. "1d", "12h"
  lastUpdated?: number;
}

// ── DNS ───────────────────────────────────────────────────────────────────────
export type DnsStrategy = 'prefer_ipv4' | 'prefer_ipv6' | 'ipv4_only' | 'ipv6_only';
export type DnsMode = 'normal' | 'fakeip';

export type ThemeMode = 'dark' | 'light' | 'system';

export interface ProxyNode {
  tag: string;
  type: string;
  server: string;
  server_port?: number;
  uuid?: string;
  id?: string;
  flow?: string;
  password?: string;
  method?: string;
  auth?: string;
  up_mbps?: number;
  down_mbps?: number;
  obfs?: {
    type?: string;
    password?: string;
  };
  congestion_control?: string;
  udp_relay_mode?: string;
  username?: string;
  version?: number;
  udp_over_tcp?: boolean;
  tls?: {
    enabled?: boolean;
    insecure?: boolean;
    server_name?: string;
    alpn?: string[];
    reality?: {
      enabled?: boolean;
      public_key?: string;
      short_id?: string;
    };
    utls?: {
      fingerprint?: string;
    };
  };
  network?: string;
  transport?: {
    type?: string;
    path?: string;
    headers?: Record<string, string>;
    service_name?: string;
  };
  [key: string]: unknown;
}

export interface NodeUsageStats {
  totalUploadBytes: number;
  totalDownloadBytes: number;
  connectionCount: number;
  firstUsedAt?: number;
  lastUsedAt?: number;
}


// ── SETTINGS ──────────────────────────────────────────────────────────────────
export interface Settings {
  // Core behavior
  proxyMode: 'system' | 'tun';
  closeToTray: boolean;
  autostart: boolean;

  // Inbound ports
  httpPort: number;
  socksPort: number;
  mixedPort: number;
  wifiSharing: boolean;
  useSeparatePorts: boolean;

  // TUN Advanced
  tunAutoRoute: boolean;
  tunAutoRedirect: boolean;
  tunStrictRoute: boolean;
  tunStack: 'system' | 'gvisor' | 'mixed';
  tunMtu: number;
  tunEndpointIndependentNat: boolean;

  // Traffic Sniffing
  sniffEnabled: boolean;
  sniffHttp: boolean;
  sniffTls: boolean;
  sniffQuic: boolean;
  sniffOverrideDestination: boolean;

  // Clash-compatible API
  apiEnabled: boolean;
  apiPort: number;
  apiSecret: string;
  apiCors: boolean;

  // DNS
  primaryDns: string;
  fallbackDns: string;
  directDns: string;
  dnsStrategy: DnsStrategy;
  dnsMode: DnsMode;
  fakeipRange: string;
  fakeipFilter: string;
  dnsLeakProtection: boolean;
  dnsCaching: boolean;

  // Routing globals
  finalOutbound: OutboundAction;
  bypassLan: boolean;

  // Legacy SNI bypass
  dnsAddress: string;
  theme: ThemeMode;
  multiplex: boolean;
}

const DEFAULT_SETTINGS: Settings = {
  proxyMode: 'tun',
  closeToTray: true,
  theme: 'dark',
  autostart: false,
  httpPort: 7890,
  socksPort: 7891,
  mixedPort: 7892,
  wifiSharing: false,
  useSeparatePorts: false,

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

  apiEnabled: true,
  apiPort: 9090,
  apiSecret: '',
  apiCors: true,

  primaryDns: 'https://1.1.1.1/dns-query',
  fallbackDns: 'https://8.8.8.8/dns-query',
  directDns: '8.8.8.8',
  dnsStrategy: 'prefer_ipv4',
  dnsMode: 'fakeip',
  fakeipRange: '198.18.0.0/15',
  fakeipFilter: 'geosite:private',
  dnsLeakProtection: true,
  dnsCaching: true,

  finalOutbound: 'proxy',
  bypassLan: true,

  dnsAddress: '',
  multiplex: false,
};

const DEFAULT_ROUTING_RULES: RoutingRule[] = [
  { id: 'r1', type: 'geoip',   value: 'private',  outbound: 'direct', invert: false, notes: 'Bypass LAN/private IPs' },
  { id: 'r2', type: 'geosite', value: 'private',  outbound: 'direct', invert: false, notes: 'Bypass private domains' },
  { id: 'r5', type: 'protocol','value': 'dns',    outbound: 'dns',    invert: false, notes: 'Route DNS queries' },
];

const DEFAULT_RULE_SETS: RuleSet[] = [];

// ── STORE INSTANCES ───────────────────────────────────────────────────────────
let settingsStore: Store | null = null;
let profilesStore: Store | null = null;
let routingStore: Store | null = null;
let statsStore: Store | null = null;

async function getSettingsStore(): Promise<Store> {
  if (!settingsStore) settingsStore = await Store.load('settings.json');
  return settingsStore;
}
async function getProfilesStore(): Promise<Store> {
  if (!profilesStore) profilesStore = await Store.load('profiles.json');
  return profilesStore;
}
async function getRoutingStore(): Promise<Store> {
  if (!routingStore) routingStore = await Store.load('routing.json');
  return routingStore;
}
async function getStatsStore(): Promise<Store> {
  if (!statsStore) statsStore = await Store.load('stats.json');
  return statsStore;
}

export const storeHelper = {
  // ── SETTINGS ──
  async getSettings(): Promise<Settings> {
    try {
      const store = await getSettingsStore();
      const entries = await store.entries<unknown>();
      const saved: Partial<Settings> = {};
      for (const [key, val] of entries) {
        if (key in DEFAULT_SETTINGS && val !== null && val !== undefined) {
          (saved as Record<string, unknown>)[key] = val;
        }
      }
      return { ...DEFAULT_SETTINGS, ...saved };
    } catch {
      return { ...DEFAULT_SETTINGS };
    }
  },

  async saveSettings(settings: Partial<Settings>): Promise<void> {
    try {
      const store = await getSettingsStore();
      const setOps = Object.entries(settings)
        .filter(([, value]) => value !== undefined)
        .map(([key, value]) => store.set(key, value));
      await Promise.all(setOps);
      // Stamp schema version on every save so migration can detect it on next boot
      await store.set('_schemaVersion', 2);
      await store.save();
    } catch (e) {
      console.error('Failed to save settings:', e);
    }
  },

  // ── ACTIVE CONFIG ──
  async getActiveConfig(): Promise<ActiveConfig | null> {
    try {
      const store = await getProfilesStore();
      return (await store.get<ActiveConfig>('activeConfig')) ?? null;
    } catch {
      return null;
    }
  },

  async saveActiveConfig(config: ActiveConfig | null): Promise<void> {
    try {
      const store = await getProfilesStore();
      await store.set('activeConfig', config);
      await store.save();
    } catch (e) {
      console.error('Failed to save active config:', e);
    }
  },

  async clearActiveConfig(): Promise<void> {
    try {
      const store = await getProfilesStore();
      await store.delete('activeConfig');
      await store.save();
    } catch (e) {
      console.error('Failed to clear active config:', e);
    }
  },

  // ── ROUTING RULES ──
  async getRoutingRules(): Promise<RoutingRule[]> {
    try {
      const store = await getRoutingStore();
      return (await store.get<RoutingRule[]>('rules')) ?? DEFAULT_ROUTING_RULES;
    } catch {
      return DEFAULT_ROUTING_RULES;
    }
  },

  async saveRoutingRules(rules: RoutingRule[]): Promise<void> {
    try {
      const store = await getRoutingStore();
      await store.set('rules', rules);
      await store.save();
    } catch (e) {
      console.error('Failed to save routing rules:', e);
    }
  },

  // ── RULE SETS ──
  async getRuleSets(): Promise<RuleSet[]> {
    try {
      const store = await getRoutingStore();
      return (await store.get<RuleSet[]>('ruleSets')) ?? DEFAULT_RULE_SETS;
    } catch {
      return DEFAULT_RULE_SETS;
    }
  },

  async saveRuleSets(sets: RuleSet[]): Promise<void> {
    try {
      const store = await getRoutingStore();
      await store.set('ruleSets', sets);
      await store.save();
    } catch (e) {
      console.error('Failed to save rule sets:', e);
    }
  },

  // ── MULTI-PROFILE MANAGEMENT ──
  async getProfiles(): Promise<Profile[]> {
    try {
      const store = await getProfilesStore();
      return (await store.get<Profile[]>('profiles')) ?? [];
    } catch {
      return [];
    }
  },

  async saveProfiles(profiles: Profile[]): Promise<void> {
    try {
      const store = await getProfilesStore();
      await store.set('profiles', profiles);
      await store.save();
    } catch (e) {
      console.error('Failed to save profiles:', e);
    }
  },

  async getActiveProfileId(): Promise<string | null> {
    try {
      const store = await getProfilesStore();
      return (await store.get<string>('activeProfileId')) ?? null;
    } catch {
      return null;
    }
  },

  async setActiveProfileId(id: string | null): Promise<void> {
    try {
      const store = await getProfilesStore();
      await store.set('activeProfileId', id);
      await store.save();
    } catch (e) {
      console.error('Failed to set active profile ID:', e);
    }
  },

  async deleteProfile(profileId: string): Promise<void> {
    const profiles = await this.getProfiles();
    const updated = profiles.filter((p) => p.id !== profileId);
    await this.saveProfiles(updated);

    const activeId = await this.getActiveProfileId();
    if (activeId === profileId) {
      // If we deleted the active profile, switch to the first remaining or null
      await this.setActiveProfileId(updated.length > 0 ? updated[0].id : null);
    }
  },

  /**
   * Migrate from the legacy single-slot ActiveConfig to the new multi-profile system.
   * Runs once on first boot after upgrade. Creates a profile from the old activeConfig
   * and clears the legacy key.
   */
  async migrateFromLegacy(): Promise<void> {
    try {
      const store = await getProfilesStore();
      const legacy = await store.get<ActiveConfig>('activeConfig');
      const existing = await store.get<Profile[]>('profiles');

      // Only migrate if there's a legacy config and no profiles exist yet
      if (legacy && (!existing || existing.length === 0)) {
        const now = Date.now();
        const migratedProfile: Profile = {
          id: crypto.randomUUID(),
          name: legacy.name,
          type: 'manual',
          importedAt: legacy.importedAt,
          lastUpdated: now,
          nodeCount: legacy.nodeCount,
          selectedNodeTag: legacy.selectedNodeTag,
        };

        await store.set('profiles', [migratedProfile]);
        await store.set('activeProfileId', migratedProfile.id);
        // Don't delete legacy key yet — keep it for rollback safety
        await store.save();
      }
    } catch (e) {
      console.error('Failed to migrate legacy config:', e);
    }
  },

  // ── SERVER STATS ──
  async getStats(): Promise<Record<string, NodeUsageStats>> {
    try {
      const store = await getStatsStore();
      return (await store.get<Record<string, NodeUsageStats>>('stats')) ?? {};
    } catch {
      return {};
    }
  },

  async saveStats(stats: Record<string, NodeUsageStats>): Promise<void> {
    try {
      const store = await getStatsStore();
      await store.set('stats', stats);
      await store.save();
    } catch (e) {
      console.error('Failed to save stats:', e);
    }
  },
};
