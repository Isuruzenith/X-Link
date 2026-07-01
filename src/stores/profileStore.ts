import { create } from 'zustand';
import { storeHelper, type Profile } from '../utils/store';
import { useLogStore } from './logStore';
import { useToastStore } from './toastStore';
import { useConnectionStore } from './connectionStore';
import { invoke } from '@tauri-apps/api/core';
import { readText } from '@tauri-apps/plugin-clipboard-manager';
import { open } from '@tauri-apps/plugin-dialog';

interface ImportResult {
  id: string;
  name: string;
  importedAt: number;
  nodeCount: number;
}

interface ProfileState {
  // Multi-profile state
  profiles: Profile[];
  activeProfileId: string | null;
  selectedProfileId: string | null;
  nodes: any[];
  selectedNodeTag: string | null;
  nodeGeoCache: Record<string, string>;

  // Import form state
  importName: string;
  importContent: string;
  importError: string | null;
  importSuccess: boolean;
  isImporting: boolean;

  // Latency testing
  latencyResults: Record<string, { latencyMs: number | null; error: string | null }>;
  isTestingLatency: boolean;

  // Computed getters
  activeProfile: () => Profile | null;

  // Actions
  initProfiles: () => Promise<void>;
  refreshNodes: (profileId?: string) => Promise<void>;
  selectNode: (node: any) => Promise<void>;
  setImportName: (name: string) => void;
  setImportContent: (content: string) => void;
  setImportError: (err: string | null) => void;
  pasteClipboard: () => Promise<void>;
  pickFileAndImport: () => Promise<void>;
  importConfig: () => Promise<void>;
  deleteProfile: (profileId: string) => Promise<void>;
  switchProfile: (profileId: string) => Promise<void>;
  testAllNodes: () => Promise<void>;
  updateNodesList: (newNodes: any[]) => void;
  selectProfile: (profileId: string) => Promise<void>;
  fetchNodeGeo: (server: string, tag: string) => Promise<void>;
}

const detectNameFromContent = (text: string): string => {
  const trimmedText = text.trim();
  if (trimmedText.startsWith('vless://') || trimmedText.startsWith('vmess://') || trimmedText.startsWith('trojan://') || trimmedText.startsWith('ss://')) {
    if (trimmedText.startsWith('vmess://')) {
      try {
        let b64 = trimmedText.replace('vmess://', '').trim();
        while (b64.length % 4 !== 0) { b64 += '='; }
        const decoded = atob(b64);
        const parsed = JSON.parse(decoded);
        if (parsed.ps) return parsed.ps;
      } catch { /* ignore decode errors */ }
    } else {
      const parts = trimmedText.split('#');
      if (parts.length > 1) return decodeURIComponent(parts[1].trim());
    }
  }
  return '';
};

const mapImportError = (raw: string): string => {
  const r = String(raw);
  if (r.includes('sing-box check failed:')) return `Configuration is invalid: ${r.replace(/.*sing-box check failed:\s*/i, '')}`;
  if (r.includes('Failed to read file:')) return 'Could not read the selected file.';
  if (r.includes('No content or file path provided')) return 'Please paste a subscription link or select a file.';
  if (r.includes('spawn_failed:')) return 'Could not start the proxy engine. Check logs for details.';
  if (r.includes('Config has no outbounds')) return 'This configuration has no proxy nodes — please import a valid subscription.';
  if (r.includes('Generated config is not valid JSON')) return 'Internal error: generated config is malformed. Please try re-importing.';
  return r;
};

export const getCountryCode = (tag: string): string | null => {
  // Check if there is a flag emoji in the tag (surrogate pairs of Regional Indicator Symbols)
  const match = tag.match(/[\uD83C][\uDDE6-\uDDFF][\uD83C][\uDDE6-\uDDFF]/);
  if (match) {
    const flag = match[0];
    const cp1 = flag.codePointAt(0);
    const cp2 = flag.codePointAt(2);
    if (cp1 && cp2) {
      const char1 = String.fromCharCode(cp1 - 127397);
      const char2 = String.fromCharCode(cp2 - 127397);
      return (char1 + char2).toLowerCase();
    }
  }

  const normalized = tag.toUpperCase();
  const mappings: Record<string, string> = {
    'UNITED STATES': 'us', 'USA': 'us', ' US ': 'us', 'US-': 'us', 'US_': 'us',
    'SINGAPORE': 'sg', ' SG ': 'sg', 'SG-': 'sg', 'SG_': 'sg',
    'JAPAN': 'jp', ' JP ': 'jp', 'JP-': 'jp', 'JP_': 'jp',
    'HONG KONG': 'hk', ' HONGKONG': 'hk', ' HK ': 'hk', 'HK-': 'hk', 'HK_': 'hk',
    'GERMANY': 'de', ' DE ': 'de', 'DE-': 'de', 'DE_': 'de',
    'UNITED KINGDOM': 'gb', ' UK ': 'gb', ' UK-': 'gb', ' GB ': 'gb', 'LONDON': 'gb',
    'FRANCE': 'fr', ' FR ': 'fr', 'FR-': 'fr', 'FR_': 'fr',
    'NETHERLANDS': 'nl', ' NL ': 'nl', 'NL-': 'nl', 'NL_': 'nl',
    'CANADA': 'ca', ' CA ': 'ca', 'CA-': 'ca', 'CA_': 'ca',
    'AUSTRALIA': 'au', ' AU ': 'au', 'AU-': 'au', 'AU_': 'au',
    'KOREA': 'kr', ' KR ': 'kr', 'KR-': 'kr', 'KR_': 'kr',
    'TAIWAN': 'tw', ' TW ': 'tw', 'TW-': 'tw', 'TW_': 'tw',
    'CHINA': 'cn', ' CN ': 'cn', 'CN-': 'cn', 'CN_': 'cn',
    'INDIA': 'in', ' IN ': 'in', 'IN-': 'in', 'IN_': 'in',
    'RUSSIA': 'ru', ' RU ': 'ru', 'RU-': 'ru', 'RU_': 'ru',
    'TURKEY': 'tr', ' TR ': 'tr', 'TR-': 'tr', 'TR_': 'tr',
    'VIETNAM': 'vn', ' VN ': 'vn', 'VN-': 'vn', 'VN_': 'vn',
    'THAILAND': 'th', ' TH ': 'th', 'TH-': 'th', 'TH_': 'th',
  };

  for (const [key, code] of Object.entries(mappings)) {
    if (normalized.includes(key)) {
      return code;
    }
  }

  // Prefix or boundaries checks
  if (normalized.startsWith('US') || normalized.endsWith('US')) return 'us';
  if (normalized.startsWith('SG') || normalized.endsWith('SG')) return 'sg';
  if (normalized.startsWith('JP') || normalized.endsWith('JP')) return 'jp';
  if (normalized.startsWith('HK') || normalized.endsWith('HK')) return 'hk';
  if (normalized.startsWith('DE') || normalized.endsWith('DE')) return 'de';
  if (normalized.startsWith('UK') || normalized.endsWith('UK') || normalized.startsWith('GB') || normalized.endsWith('GB')) return 'gb';
  if (normalized.startsWith('FR') || normalized.endsWith('FR')) return 'fr';
  if (normalized.startsWith('NL') || normalized.endsWith('NL')) return 'nl';
  if (normalized.startsWith('CA') || normalized.endsWith('CA')) return 'ca';
  if (normalized.startsWith('AU') || normalized.endsWith('AU')) return 'au';
  if (normalized.startsWith('KR') || normalized.endsWith('KR')) return 'kr';
  if (normalized.startsWith('TW') || normalized.endsWith('TW')) return 'tw';
  if (normalized.startsWith('CN') || normalized.endsWith('CN')) return 'cn';
  if (normalized.startsWith('IN') || normalized.endsWith('IN')) return 'in';
  if (normalized.startsWith('RU') || normalized.endsWith('RU')) return 'ru';
  if (normalized.startsWith('TR') || normalized.endsWith('TR')) return 'tr';
  if (normalized.startsWith('VN') || normalized.endsWith('VN')) return 'vn';
  if (normalized.startsWith('TH') || normalized.endsWith('TH')) return 'th';

  return null;
};

export const useProfileStore = create<ProfileState>((set, get) => ({
  profiles: [],
  activeProfileId: null,
  selectedProfileId: null,
  nodes: [],
  selectedNodeTag: null,
  nodeGeoCache: {},
  importName: '',
  importContent: '',
  importError: null,
  importSuccess: false,
  isImporting: false,
  latencyResults: {},
  isTestingLatency: false,

  activeProfile: () => {
    const { profiles, activeProfileId } = get();
    return profiles.find((p) => p.id === activeProfileId) ?? null;
  },

  initProfiles: async () => {
    // Run legacy migration first (converts old single-slot config to multi-profile)
    await storeHelper.migrateFromLegacy();

    const profiles = await storeHelper.getProfiles();
    const activeProfileId = await storeHelper.getActiveProfileId();
    set({ profiles, activeProfileId, selectedProfileId: activeProfileId });

    if (activeProfileId) {
      await get().refreshNodes(activeProfileId);
    }
  },

  refreshNodes: async (profileId) => {
    const pid = profileId || get().selectedProfileId || get().activeProfileId;
    if (!pid) {
      set({ nodes: [], selectedNodeTag: null });
      return;
    }

    try {
      // Fetch outbounds for the specific profile (or fall back to active config)
      const outbounds = await invoke<any[]>('get_profile_outbounds', { profileId: pid });
      set({ nodes: outbounds || [] });

      const profile = get().profiles.find((p) => p.id === pid);
      let tag = profile?.selectedNodeTag ?? null;

      if (!tag || !(outbounds || []).some((n) => n.tag === tag)) {
        try {
          const active = await invoke<any>('get_active_outbound');
          tag = active?.tag || outbounds?.[0]?.tag || null;
        } catch {
          tag = outbounds?.[0]?.tag || null;
        }
      }

      set({ selectedNodeTag: tag });

      // Sync tag back to profile metadata
      if (profile && tag !== profile.selectedNodeTag) {
        const updatedProfiles = get().profiles.map((p) =>
          p.id === pid ? { ...p, selectedNodeTag: tag } : p
        );
        set({ profiles: updatedProfiles });
        await storeHelper.saveProfiles(updatedProfiles);
      }
    } catch {
      set({ nodes: [] });
    }
  },

  selectNode: async (node) => {
    const { selectedProfileId, activeProfileId, profiles } = get();
    const targetProfileId = selectedProfileId || activeProfileId;
    const logStore = useLogStore.getState();
    const connectionStore = useConnectionStore.getState();

    set({ selectedNodeTag: node.tag });

    // Update the target profile's selectedNodeTag
    if (targetProfileId) {
      const updatedProfiles = profiles.map((p) =>
        p.id === targetProfileId ? { ...p, selectedNodeTag: node.tag } : p
      );
      set({ profiles: updatedProfiles });
      await storeHelper.saveProfiles(updatedProfiles);
    }

    // Node-wise profile activation
    if (targetProfileId && targetProfileId !== activeProfileId) {
      try {
        logStore.pushSystemLog(`Activating profile "${profiles.find((p) => p.id === targetProfileId)?.name}"...`);
        await invoke('switch_profile', { profileId: targetProfileId, selectedNodeTag: node.tag });
        await storeHelper.setActiveProfileId(targetProfileId);
        set({ activeProfileId: targetProfileId });
      } catch (err) {
        logStore.pushSystemLog(`Failed to switch profile: ${err}`);
      }
    } else if (connectionStore.isConnected) {
      try {
        logStore.pushSystemLog(`Switching active node to "${node.tag}"...`);
        await invoke('switch_node_hot', { tag: node.tag });
        logStore.pushSystemLog(`Successfully switched to "${node.tag}".`);
      } catch (err) {
        logStore.pushSystemLog(`Failed to switch node: ${err}`);
      }
    }
  },

  setImportName: (name) => set({ importName: name }),
  setImportContent: (content) => {
    set({ importContent: content });
    if (!get().importName.trim()) {
      const tagPart = detectNameFromContent(content);
      if (tagPart) set({ importName: tagPart });
    }
  },
  setImportError: (err) => set({ importError: err }),

  pasteClipboard: async () => {
    const logStore = useLogStore.getState();
    try {
      const text = await readText();
      if (text) {
        get().setImportContent(text);
        logStore.pushSystemLog('Pasted config from clipboard.');
      } else {
        set({ importError: 'Clipboard is empty.' });
      }
    } catch {
      set({ importError: 'Failed to read clipboard.' });
    }
  },

  pickFileAndImport: async () => {
    set({ importError: null, importSuccess: false });
    const logStore = useLogStore.getState();
    try {
      const path = await open({
        multiple: false,
        filters: [{ name: 'Config', extensions: ['json', 'yaml', 'yml', 'txt'] }],
      });
      if (!path || typeof path !== 'string') return;
      set({ isImporting: true });
      const fileName = path.split(/[\\/]/).pop() || 'Imported Config';
      const name = get().importName.trim() || fileName.replace(/\.[^/.]+$/, '');
      const profileId = crypto.randomUUID();

      const result = await invoke<ImportResult>('import_config', { filePath: path, name, profileId });

      const newProfile: Profile = {
        id: result.id,
        name: result.name,
        type: 'file',
        importedAt: result.importedAt,
        lastUpdated: result.importedAt,
        nodeCount: result.nodeCount,
        selectedNodeTag: null,
      };

      const updatedProfiles = [...get().profiles, newProfile];
      await storeHelper.saveProfiles(updatedProfiles);
      await storeHelper.setActiveProfileId(newProfile.id);

      set({
        profiles: updatedProfiles,
        activeProfileId: newProfile.id,
        selectedProfileId: newProfile.id,
        importSuccess: true,
        importName: '',
        importContent: '',
      });

      logStore.pushSystemLog(`Profile "${result.name}" imported! ${result.nodeCount} nodes.`);
      useToastStore.getState().addToast('success', `Profile "${result.name}" imported with ${result.nodeCount} nodes.`);
      setTimeout(() => set({ importSuccess: false }), 3000);
      await get().refreshNodes(newProfile.id);
    } catch (err) {
      set({ importError: mapImportError(String(err)) });
    } finally {
      set({ isImporting: false });
    }
  },

  importConfig: async () => {
    set({ importError: null, importSuccess: false, isImporting: true });
    const { importContent, importName } = get();
    const logStore = useLogStore.getState();

    if (!importContent.trim()) {
      set({ importError: 'Please paste config content.', isImporting: false });
      return;
    }

    try {
      const name = importName.trim() || 'My Config';
      const profileId = crypto.randomUUID();

      // Detect if the content is a subscription URL
      const isUrl = importContent.trim().startsWith('http://') || importContent.trim().startsWith('https://');
      const profileType: Profile['type'] = isUrl ? 'subscription' : 'manual';

      const result = await invoke<ImportResult>('import_config', { content: importContent, name, profileId });

      const newProfile: Profile = {
        id: result.id,
        name: result.name,
        type: profileType,
        subscriptionUrl: isUrl ? importContent.trim() : undefined,
        importedAt: result.importedAt,
        lastUpdated: result.importedAt,
        nodeCount: result.nodeCount,
        selectedNodeTag: null,
      };

      const updatedProfiles = [...get().profiles, newProfile];
      await storeHelper.saveProfiles(updatedProfiles);
      await storeHelper.setActiveProfileId(newProfile.id);

      set({
        profiles: updatedProfiles,
        activeProfileId: newProfile.id,
        selectedProfileId: newProfile.id,
        importSuccess: true,
        importName: '',
        importContent: '',
      });

      logStore.pushSystemLog(`Profile "${result.name}" imported! ${result.nodeCount} nodes.`);
      useToastStore.getState().addToast('success', `Profile "${result.name}" imported with ${result.nodeCount} nodes.`);
      setTimeout(() => set({ importSuccess: false }), 3000);
      await get().refreshNodes(newProfile.id);
    } catch (err) {
      set({ importError: mapImportError(String(err)) });
    } finally {
      set({ isImporting: false });
    }
  },

  deleteProfile: async (profileId) => {
    const logStore = useLogStore.getState();
    const connectionStore = useConnectionStore.getState();
    const { profiles, activeProfileId } = get();

    const profile = profiles.find((p) => p.id === profileId);
    if (!profile) return;

    // If deleting the active profile while connected, disconnect first
    if (profileId === activeProfileId && connectionStore.isConnected) {
      try {
        await invoke('toggle_proxy', { enable: false });
      } catch { /* ignore disconnect errors */ }
    }

    // Remove from backend
    try {
      await invoke('delete_profile_config', { profileId });
    } catch { /* ignore if file doesn't exist */ }

    // Remove from store
    await storeHelper.deleteProfile(profileId);
    const updatedProfiles = profiles.filter((p) => p.id !== profileId);

    const newActiveId = profileId === activeProfileId
      ? (updatedProfiles.length > 0 ? updatedProfiles[0].id : null)
      : activeProfileId;

    const { selectedProfileId } = get();
    const newSelectedId = profileId === selectedProfileId
      ? newActiveId
      : selectedProfileId;

    set({ profiles: updatedProfiles, activeProfileId: newActiveId, selectedProfileId: newSelectedId });

    // If we switched to a new active profile, load its nodes
    if (newActiveId && newActiveId !== activeProfileId) {
      try {
        await invoke('switch_profile', { profileId: newActiveId });
      } catch { /* ignore */ }
    }

    if (newSelectedId) {
      await get().refreshNodes(newSelectedId);
    } else {
      set({ nodes: [], selectedNodeTag: null });
    }

    logStore.pushSystemLog(`Profile "${profile.name}" deleted.`);
  },

  switchProfile: async (profileId) => {
    const logStore = useLogStore.getState();
    const { profiles } = get();
    const profile = profiles.find((p) => p.id === profileId);
    if (!profile) return;

    try {
      await invoke('switch_profile', { profileId, selectedNodeTag: profile.selectedNodeTag });
      await storeHelper.setActiveProfileId(profileId);
      set({ activeProfileId: profileId, selectedProfileId: profileId });
      logStore.pushSystemLog(`Switched to profile "${profile.name}".`);
      await get().refreshNodes(profileId);
    } catch (err) {
      logStore.pushSystemLog(`Failed to switch profile: ${err}`);
    }
  },

  selectProfile: async (profileId) => {
    set({ selectedProfileId: profileId });
    await get().refreshNodes(profileId);
  },

  testAllNodes: async () => {
    const pid = get().selectedProfileId || get().activeProfileId;
    const logStore = useLogStore.getState();
    if (!pid) return;

    set({ isTestingLatency: true, latencyResults: {} });
    logStore.pushSystemLog('Testing latency for all nodes...');

    try {
      const results = await invoke<Array<{ tag: string; latencyMs: number | null; error: string | null }>>(
        'test_all_nodes',
        { profileId: pid }
      );

      const latencyMap: Record<string, { latencyMs: number | null; error: string | null }> = {};
      for (const r of results) {
        latencyMap[r.tag] = { latencyMs: r.latencyMs, error: r.error };
      }

      set({ latencyResults: latencyMap });

      const successful = results.filter((r) => r.latencyMs !== null).length;
      logStore.pushSystemLog(`Latency test complete: ${successful}/${results.length} nodes reachable.`);
    } catch (err) {
      logStore.pushSystemLog(`Latency test failed: ${err}`);
    } finally {
      set({ isTestingLatency: false });
    }
  },

  updateNodesList: (newNodes) => set({ nodes: newNodes }),

  fetchNodeGeo: async (server, tag) => {
    if (!server) return;
    const { nodeGeoCache } = get();
    if (nodeGeoCache[server]) return;

    // Set loading placeholder
    set((state) => ({
      nodeGeoCache: { ...state.nodeGeoCache, [server]: 'loading' }
    }));

    try {
      const res = await fetch(`https://freeipapi.com/api/json/${server}`);
      if (!res.ok) throw new Error('API failed');
      const data = await res.json();
      if (data && data.countryCode && data.countryCode.length === 2) {
        const code = data.countryCode.toLowerCase();
        set((state) => ({
          nodeGeoCache: { ...state.nodeGeoCache, [server]: code }
        }));
      } else {
        throw new Error('Invalid response');
      }
    } catch {
      // Fallback to tag parsing
      const fallbackCode = getCountryCode(tag);
      set((state) => ({
        nodeGeoCache: { ...state.nodeGeoCache, [server]: fallbackCode || 'unknown' }
      }));
    }
  },
}));
