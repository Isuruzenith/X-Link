import { Store } from '@tauri-apps/plugin-store';

// ── PROFILE ──────────────────────────────────────────────────────────────────
export interface Profile {
  id: string;
  name: string;
  type: 'url' | 'file' | 'manual';
  subscriptionUrl?: string;
  configPath: string;
  lastUpdated: number;
  nodeCount: number;
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
export type DnsRuleType = 'domain' | 'domain_suffix' | 'domain_keyword' | 'geosite' | 'rule_set' | 'ip_cidr';
export type DnsStrategy = 'prefer_ipv4' | 'prefer_ipv6' | 'ipv4_only' | 'ipv6_only';
export type DnsMode = 'normal' | 'fakeip';

export interface DnsRule {
  id: string;
  type: DnsRuleType;
  value: string;
  server: string;   // 'primary' | 'fallback' | 'direct' | 'block'
  disableCache: boolean;
  invert: boolean;
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

  // Multiplexing
  muxEnabled: boolean;
  muxProtocol: 'smux' | 'yamux' | 'h2mux';
  muxMaxConnections: number;
  muxMinStreams: number;
  muxMaxStreams: number;
  muxPadding: boolean;
  muxBrutal: boolean;
  muxBrutalUpMbps: number;
  muxBrutalDownMbps: number;

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
  bypassChina: boolean;

  // Legacy SNI bypass
  dnsAddress: string;
  sniHost: string;
}

const DEFAULT_SETTINGS: Settings = {
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
  tunStack: 'mixed',
  tunMtu: 9000,
  tunEndpointIndependentNat: false,

  sniffEnabled: true,
  sniffHttp: true,
  sniffTls: true,
  sniffQuic: true,
  sniffOverrideDestination: false,

  muxEnabled: false,
  muxProtocol: 'h2mux',
  muxMaxConnections: 4,
  muxMinStreams: 4,
  muxMaxStreams: 0,
  muxPadding: false,
  muxBrutal: false,
  muxBrutalUpMbps: 100,
  muxBrutalDownMbps: 100,

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
  bypassChina: false,

  dnsAddress: '',
  sniHost: 'aka.ms',
};

const DEFAULT_ROUTING_RULES: RoutingRule[] = [
  { id: 'r1', type: 'geoip',   value: 'private',  outbound: 'direct', invert: false, notes: 'Bypass LAN/private IPs' },
  { id: 'r2', type: 'geosite', value: 'private',  outbound: 'direct', invert: false, notes: 'Bypass private domains' },
  { id: 'r3', type: 'geoip',   value: 'cn',       outbound: 'direct', invert: false, notes: 'China IPs - direct' },
  { id: 'r4', type: 'geosite', value: 'cn',       outbound: 'direct', invert: false, notes: 'China sites - direct' },
  { id: 'r5', type: 'protocol','value': 'dns',    outbound: 'dns',    invert: false, notes: 'Route DNS queries' },
];

const DEFAULT_RULE_SETS: RuleSet[] = [
  { id: 'rs1', tag: 'geoip-cn',     type: 'remote', format: 'binary', url: 'https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geoip/cn.srs',     updateInterval: '1d' },
  { id: 'rs2', tag: 'geosite-cn',   type: 'remote', format: 'binary', url: 'https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geosite/cn.srs',    updateInterval: '1d' },
  { id: 'rs3', tag: 'geosite-private', type: 'remote', format: 'binary', url: 'https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geosite/private.srs', updateInterval: '7d' },
];

const DEFAULT_DNS_RULES: DnsRule[] = [
  { id: 'd1', type: 'geosite', value: 'cn',      server: 'direct', disableCache: false, invert: false },
  { id: 'd2', type: 'geosite', value: 'private', server: 'direct', disableCache: false, invert: false },
];

// ── STORE INSTANCES ───────────────────────────────────────────────────────────
let settingsStore: Store | null = null;
let profilesStore: Store | null = null;
let routingStore: Store | null = null;
let dnsStore: Store | null = null;

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
async function getDnsStore(): Promise<Store> {
  if (!dnsStore) dnsStore = await Store.load('dns.json');
  return dnsStore;
}

export const storeHelper = {
  // ── SETTINGS ──
  async getSettings(): Promise<Settings> {
    try {
      const store = await getSettingsStore();
      const saved: Partial<Settings> = {};
      for (const key of Object.keys(DEFAULT_SETTINGS) as (keyof Settings)[]) {
        const val = await store.get<Settings[typeof key]>(key as string);
        if (val !== null && val !== undefined) (saved as any)[key] = val;
      }
      return { ...DEFAULT_SETTINGS, ...saved };
    } catch {
      return { ...DEFAULT_SETTINGS };
    }
  },

  async saveSettings(settings: Partial<Settings>): Promise<void> {
    try {
      const store = await getSettingsStore();
      for (const [key, value] of Object.entries(settings)) {
        if (value !== undefined) await store.set(key, value);
      }
      await store.save();
    } catch (e) {
      console.error('Failed to save settings:', e);
    }
  },

  // ── PROFILES ──
  async getProfiles(): Promise<Profile[]> {
    try {
      const store = await getProfilesStore();
      return (await store.get<Profile[]>('profiles')) || [];
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

  async addProfile(profile: Profile): Promise<void> {
    const list = await this.getProfiles();
    const filtered = list.filter((p) => p.id !== profile.id);
    filtered.push(profile);
    await this.saveProfiles(filtered);
  },

  async removeProfile(profileId: string): Promise<void> {
    const list = await this.getProfiles();
    await this.saveProfiles(list.filter((p) => p.id !== profileId));
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

  // ── DNS RULES ──
  async getDnsRules(): Promise<DnsRule[]> {
    try {
      const store = await getDnsStore();
      return (await store.get<DnsRule[]>('rules')) ?? DEFAULT_DNS_RULES;
    } catch {
      return DEFAULT_DNS_RULES;
    }
  },

  async saveDnsRules(rules: DnsRule[]): Promise<void> {
    try {
      const store = await getDnsStore();
      await store.set('rules', rules);
      await store.save();
    } catch (e) {
      console.error('Failed to save DNS rules:', e);
    }
  },
};
