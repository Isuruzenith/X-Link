import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { readText } from '@tauri-apps/plugin-clipboard-manager';
import { ask } from '@tauri-apps/plugin-dialog';
import { NavRail } from './components/NavRail';
import type { TabId } from './components/NavRail';
import { DashboardView } from './views/DashboardView';
import { ProfilesView } from './views/ProfilesView';
import { RoutingView } from './views/RoutingView';
import { DnsView } from './views/DnsView';
import { LogsView } from './views/LogsView';
import { SettingsView } from './views/SettingsView';
import { NodeEditor } from './components/domain/NodeEditor';
import { storeHelper } from './utils/store';
import type {
  Profile, Settings, RoutingRule, RuleSet, DnsRule,
} from './utils/store';

const uid = () => Math.random().toString(36).slice(2, 10);

export default function App() {
  // ── NAVIGATION ────────────────────────────────────────────────────────────
  const [activeTab, setActiveTab] = useState<TabId>('dashboard');

  // ── APP CORE STATE ─────────────────────────────────────────────────────────
  const [isConnected, setIsConnected] = useState(false);
  const [activeProfileId, setActiveProfileId] = useState<string | null>(null);
  const [uptime, setUptime] = useState(0);
  const [httpPort, setHttpPort] = useState(7890);
  const [socksPort, setSocksPort] = useState(7891);
  const [mixedPort, setMixedPort] = useState(7892);
  const [isElevated, setIsElevated] = useState(false);
  const [singboxVersion, setSingboxVersion] = useState('Unknown');
  const [appVersion, setAppVersion] = useState('0.1.0');

  // ── TRAFFIC ───────────────────────────────────────────────────────────────
  const [uploadBytes, setUploadBytes] = useState(0);
  const [downloadBytes, setDownloadBytes] = useState(0);
  const [uploadSpeed, setUploadSpeed] = useState(0);
  const [downloadSpeed, setDownloadSpeed] = useState(0);
  const [activeConnections, setActiveConnections] = useState(0);
  const [speedHistory, setSpeedHistory] = useState<{ up: number; down: number }[]>([]);

  // ── PROFILES ───────────────────────────────────────────────────────────────
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [selectedProfileId, setSelectedProfileId] = useState<string | null>(null);
  const [profileOutbounds, setProfileOutbounds] = useState<any[]>([]);
  const [selectedOutboundTag, setSelectedOutboundTag] = useState<string | null>(null);
  const [importName, setImportName] = useState('');
  const [importContent, setImportContent] = useState('');
  const [importError, setImportError] = useState<string | null>(null);
  const [importSuccess, setImportSuccess] = useState(false);
  const [isImporting, setIsImporting] = useState(false);

  // ── SETTINGS ───────────────────────────────────────────────────────────────
  const [settings, setSettings] = useState<Settings>({
    proxyMode: 'tun', closeToTray: true, autostart: false,
    httpPort: 7890, socksPort: 7891, mixedPort: 7892, wifiSharing: false,
    tunAutoRoute: true, tunAutoRedirect: false, tunStrictRoute: true,
    tunStack: 'mixed', tunMtu: 9000, tunEndpointIndependentNat: false,
    sniffEnabled: true, sniffHttp: true, sniffTls: true, sniffQuic: true, sniffOverrideDestination: false,
    muxEnabled: false, muxProtocol: 'h2mux', muxMaxConnections: 4,
    muxMinStreams: 4, muxMaxStreams: 0, muxPadding: false, muxBrutal: false,
    muxBrutalUpMbps: 100, muxBrutalDownMbps: 100,
    apiEnabled: false, apiPort: 9090, apiSecret: '', apiCors: true,
    primaryDns: 'https://1.1.1.1/dns-query', fallbackDns: 'https://8.8.8.8/dns-query',
    directDns: '223.5.5.5', dnsStrategy: 'prefer_ipv4', dnsMode: 'fakeip',
    fakeipRange: '198.18.0.0/15', fakeipFilter: 'geosite:private',
    dnsLeakProtection: true, dnsCaching: true,
    finalOutbound: 'proxy', bypassLan: true, bypassChina: false,
    dnsAddress: '', sniHost: '',
  });
  const [conflictingPorts, setConflictingPorts] = useState<number[]>([]);
  const [settingsTab, setSettingsTab] = useState<'general' | 'tun' | 'sniff' | 'mux' | 'api'>('general');

  // ── ROUTING ────────────────────────────────────────────────────────────────
  const [routingRules, setRoutingRules] = useState<RoutingRule[]>([]);
  const [ruleSets, setRuleSets] = useState<RuleSet[]>([]);
  const [showAddRule, setShowAddRule] = useState(false);
  const [editingRule, setEditingRule] = useState<RoutingRule | null>(null);
  const [ruleForm, setRuleForm] = useState<Omit<RoutingRule, 'id'>>({ type: 'geoip', value: '', outbound: 'direct', invert: false, notes: '' });
  const [showAddRuleSet, setShowAddRuleSet] = useState(false);
  const [ruleSetForm, setRuleSetForm] = useState<Omit<RuleSet, 'id'>>({ tag: '', type: 'remote', format: 'binary', url: '', updateInterval: '1d' });
  const [updatingRuleSet, setUpdatingRuleSet] = useState<string | null>(null);

  // ── DNS ────────────────────────────────────────────────────────────────────
  const [dnsRules, setDnsRules] = useState<DnsRule[]>([]);
  const [showAddDnsRule, setShowAddDnsRule] = useState(false);
  const [editingDnsRule, setEditingDnsRule] = useState<DnsRule | null>(null);
  const [dnsRuleForm, setDnsRuleForm] = useState<Omit<DnsRule, 'id'>>({ type: 'geosite', value: '', server: 'direct', disableCache: false, invert: false });

  // ── LOGS ───────────────────────────────────────────────────────────────────
  const [logs, setLogs] = useState<{ type: 'info' | 'warn' | 'error' | 'system'; text: string }[]>([]);
  const [autoScrollLogs, setAutoScrollLogs] = useState(true);

  // ── NODE EDITOR ────────────────────────────────────────────────────────────
  const [isEditingConfig, setIsEditingConfig] = useState(false);
  const [editTag, setEditTag] = useState('');
  const [editAddress, setEditAddress] = useState('');
  const [editPort, setEditPort] = useState(443);
  const [editProtocol, setEditProtocol] = useState('vless');
  const [editUuid, setEditUuid] = useState('');
  const [editFlow, setEditFlow] = useState('');
  const [editPassword, setEditPassword] = useState('');
  const [editMethod, setEditMethod] = useState('aes-256-gcm');
  const [editNetwork, setEditNetwork] = useState<string>('tcp');
  const [editHeaderType, setEditHeaderType] = useState('');
  const [editPath, setEditPath] = useState('');
  const [editHost, setEditHost] = useState('');
  const [editServiceName, setEditServiceName] = useState('');
  const [editTlsEnabled, setEditTlsEnabled] = useState(false);
  const [editAllowInsecure, setEditAllowInsecure] = useState(false);
  const [editServerName, setEditServerName] = useState('');
  const [editAlpn, setEditAlpn] = useState('');
  const [editRealityEnabled, setEditRealityEnabled] = useState(false);
  const [editFingerprint, setEditFingerprint] = useState('chrome');
  const [editPublicKey, setEditPublicKey] = useState('');
  const [editShortId, setEditShortId] = useState('');
  const [editHy2Auth, setEditHy2Auth] = useState('');
  const [editHy2UpBw, setEditHy2UpBw] = useState('100 mbps');
  const [editHy2DownBw, setEditHy2DownBw] = useState('200 mbps');
  const [editHy2ObfsType, setEditHy2ObfsType] = useState('');
  const [editHy2ObfsPassword, setEditHy2ObfsPassword] = useState('');
  const [editTuicUuid, setEditTuicUuid] = useState('');
  const [editTuicPassword, setEditTuicPassword] = useState('');
  const [editTuicCongestion, setEditTuicCongestion] = useState('bbr');
  const [editTuicUdpMode, setEditTuicUdpMode] = useState('native');
  const [editWgSecretKey, setEditWgSecretKey] = useState('');
  const [editWgPeerPublicKey, setEditWgPeerPublicKey] = useState('');
  const [editWgPreSharedKey, setEditWgPreSharedKey] = useState('');
  const [editWgEndpoint, setEditWgEndpoint] = useState('');
  const [editWgAllowedIps, setEditWgAllowedIps] = useState('0.0.0.0/0, ::/0');
  const [editWgReserved, setEditWgReserved] = useState('');
  const [editWgMtu, setEditWgMtu] = useState(1280);
  const [editSshUser, setEditSshUser] = useState('root');
  const [editSshPassword, setEditSshPassword] = useState('');
  const [editSshPrivateKey, setEditSshPrivateKey] = useState('');
  const [editSshHostKey, setEditSshHostKey] = useState('');
  const [editSocksUser, setEditSocksUser] = useState('');
  const [editSocksPassword, setEditSocksPassword] = useState('');
  const [editSocksVersion, setEditSocksVersion] = useState(5);
  const [editSocksUdpOverTcp, setEditSocksUdpOverTcp] = useState(false);
  const [editHttpUser, setEditHttpUser] = useState('');
  const [editHttpPassword, setEditHttpPassword] = useState('');
  const [editHttpTls, setEditHttpTls] = useState(false);
  const [editSaveError, setEditSaveError] = useState<string | null>(null);
  const [isSavingConfig, setIsSavingConfig] = useState(false);
  const [editorSection, setEditorSection] = useState<'basic' | 'transport' | 'tls'>('basic');

  // ── INITIALIZATION ─────────────────────────────────────────────────────────
  useEffect(() => {
    (async () => {
      try {
        setIsElevated(await invoke<boolean>('check_tun_support'));
        setAppVersion(await invoke<string>('get_app_version'));
        setSingboxVersion(await invoke<string>('get_singbox_version'));
      } catch { }

      const savedSettings = await storeHelper.getSettings();
      setSettings(savedSettings);
      setHttpPort(savedSettings.httpPort);
      setSocksPort(savedSettings.socksPort);
      setMixedPort(savedSettings.mixedPort);

      const savedProfiles = await storeHelper.getProfiles();
      setProfiles(savedProfiles);
      if (savedProfiles.length > 0) setSelectedProfileId(savedProfiles[0].id);

      setRoutingRules(await storeHelper.getRoutingRules());
      setRuleSets(await storeHelper.getRuleSets());
      setDnsRules(await storeHelper.getDnsRules());
    })();

    pushSystemLog('X-Link Core initialized.');
    pushSystemLog('Ready to establish proxy tunnels.');

    const unlistenLog = listen<string>('sing-box-log', (e) => {
      const line = e.payload.trim();
      if (!line) return;
      let type: 'info' | 'warn' | 'error' | 'system' = 'info';
      if (line.toLowerCase().includes('warn')) type = 'warn';
      else if (line.toLowerCase().includes('err') || line.toLowerCase().includes('fatal')) type = 'error';
      setLogs((prev) => [...prev, { type, text: line }].slice(-500));
    });

    const unlistenTerm = listen<number | null>('sing-box-terminated', (e) => {
      setIsConnected(false);
      setActiveProfileId(null);
      pushSystemLog(`sing-box terminated (code ${e.payload ?? 'None'})`);
    });

    (async () => {
      try {
        const buffered = await invoke<string[]>('get_buffered_logs');
        if (buffered?.length) {
          setLogs((prev) => [...prev, ...buffered.map((line) => {
            let type: 'info' | 'warn' | 'error' | 'system' = 'info';
            if (line.toLowerCase().includes('warn')) type = 'warn';
            else if (line.toLowerCase().includes('err')) type = 'error';
            return { type, text: line };
          })].slice(-500));
        }
      } catch { }
    })();

    return () => {
      unlistenLog.then((f) => f());
      unlistenTerm.then((f) => f());
    };
  }, []);

  useEffect(() => {
    (async () => {
      const ports = [httpPort, socksPort, mixedPort];
      const conflicts: number[] = [];
      for (const p of ports) {
        try {
          const res = await invoke<number | null>('check_port_conflict', { ports: [p] });
          if (res) conflicts.push(p);
        } catch { }
      }
      setConflictingPorts(conflicts);
    })();
  }, [activeTab, httpPort, socksPort, mixedPort]);

  useEffect(() => {
    const poll = setInterval(async () => {
      try {
        const status = await invoke<{ status: string; activeProfileId: string | null; httpPort: number; socksPort: number; mixedPort: number; tunEnabled: boolean; uptime: number }>('get_proxy_status');
        const active = status.status === 'connected';
        setIsConnected(active);
        setActiveProfileId(status.activeProfileId);
        setUptime(status.uptime);
        if (active) {
          const stats = await invoke<{ uploadBytes: number; downloadBytes: number; uploadSpeed: number; downloadSpeed: number; activeConnections: number }>('get_traffic_stats');
          setUploadBytes(stats.uploadBytes);
          setDownloadBytes(stats.downloadBytes);
          setUploadSpeed(stats.uploadSpeed);
          setDownloadSpeed(stats.downloadSpeed);
          setActiveConnections(stats.activeConnections);
          setSpeedHistory((prev) => [...prev, { up: stats.uploadSpeed, down: stats.downloadSpeed }].slice(-30));
        } else {
          setUploadSpeed(0); setDownloadSpeed(0); setActiveConnections(0);
          setSpeedHistory((prev) => [...prev, { up: 0, down: 0 }].slice(-30));
        }
      } catch { }
    }, 1000);
    return () => clearInterval(poll);
  }, []);

  useEffect(() => {
    if (!selectedProfileId) { setProfileOutbounds([]); setSelectedOutboundTag(null); return; }
    (async () => {
      try {
        const outbounds = await invoke<any[]>('get_profile_outbounds', { profileId: selectedProfileId });
        setProfileOutbounds(outbounds || []);
        const active = await invoke<any>('get_profile_outbound', { profileId: selectedProfileId });
        if (active) setSelectedOutboundTag(active.tag);
      } catch { setProfileOutbounds([]); }
    })();
  }, [selectedProfileId, profiles]);

  useEffect(() => {
    if (profiles.length > 0 && !selectedProfileId) setSelectedProfileId(profiles[0].id);
  }, [profiles, selectedProfileId]);

  // ── HELPERS ────────────────────────────────────────────────────────────────
  const pushSystemLog = (text: string) =>
    setLogs((prev) => [...prev, { type: 'system' as const, text: `[System] ${text}` }].slice(-500));

  // ── ACTIONS ────────────────────────────────────────────────────────────────
  const handleToggleConnect = async () => {
    if (isConnected) {
      try {
        const result = await invoke<string>('toggle_proxy', { start: false, profileId: '' });
        if (result === 'stopped') { setIsConnected(false); setActiveProfileId(null); pushSystemLog('Proxy tunnels stopped.'); }
      } catch (e) { pushSystemLog(`Error stopping proxy: ${e}`); }
    } else {
      if (!selectedProfileId) { pushSystemLog('Error: Select or import a profile first.'); setActiveTab('profiles'); return; }
      try {
        const conflict = await invoke<number | null>('check_port_conflict', { ports: [httpPort, socksPort, mixedPort] });
        if (conflict) { pushSystemLog(`Error: Port ${conflict} is in use.`); setActiveTab('settings'); return; }
      } catch { }
      const target = profiles.find((p) => p.id === selectedProfileId);
      pushSystemLog(`Booting X-Link Core using profile "${target?.name || 'Default'}"...`);
      try {
        const result = await invoke<string>('toggle_proxy', { start: true, profileId: selectedProfileId });
        if (result === 'started') {
          setIsConnected(true); setActiveProfileId(selectedProfileId);
          pushSystemLog(`sing-box established on port Mixed:${mixedPort}.`);
        }
      } catch (e) { pushSystemLog(`Startup error: ${e}`); }
    }
  };

  const handleRequestElevation = async () => {
    try { await invoke('request_elevation'); }
    catch (e) { pushSystemLog(`Elevation aborted: ${e}`); }
  };

  const handleDeleteProfile = async (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await invoke('delete_profile', { profileId: id });
      await storeHelper.removeProfile(id);
      const remaining = profiles.filter((p) => p.id !== id);
      setProfiles(remaining);
      if (selectedProfileId === id) setSelectedProfileId(remaining.length > 0 ? remaining[0].id : null);
    } catch (err) { pushSystemLog(`Failed to delete profile: ${err}`); }
  };

  const handleSelectOutbound = async (node: any) => {
    if (!selectedProfileId) return;
    try {
      await invoke('update_profile_config', { profileId: selectedProfileId, newOutbound: node });
      setSelectedOutboundTag(node.tag);
      pushSystemLog(`Node switched to "${node.tag}".`);
    } catch (err) { pushSystemLog(`Failed to switch node: ${err}`); }
  };

  const handlePasteClipboard = async () => {
    try {
      const text = await readText();
      if (text) {
        setImportContent(text);
        pushSystemLog('Pasted config from clipboard.');
        
        // Auto-detect profile name from URI fragment
        if (!importName.trim()) {
          const trimmedText = text.trim();
          if (trimmedText.startsWith('vless://') || trimmedText.startsWith('vmess://') || trimmedText.startsWith('trojan://') || trimmedText.startsWith('ss://')) {
            let tagPart = '';
            if (trimmedText.startsWith('vmess://')) {
              try {
                let b64 = trimmedText.replace('vmess://', '').trim();
                while (b64.length % 4 !== 0) { b64 += '='; }
                const decoded = atob(b64);
                const parsed = JSON.parse(decoded);
                if (parsed.ps) {
                  tagPart = parsed.ps;
                }
              } catch {}
            } else {
              const parts = trimmedText.split('#');
              if (parts.length > 1) {
                tagPart = decodeURIComponent(parts[1].trim());
              }
            }
            if (tagPart) {
              setImportName(tagPart);
            }
          }
        }
      }
      else setImportError('Clipboard is empty.');
    } catch { setImportError('Failed to read clipboard.'); }
  };

  const handleImportContentChange = (val: string) => {
    setImportContent(val);
    if (!importName.trim()) {
      const trimmedText = val.trim();
      if (trimmedText.startsWith('vless://') || trimmedText.startsWith('vmess://') || trimmedText.startsWith('trojan://') || trimmedText.startsWith('ss://')) {
        let tagPart = '';
        if (trimmedText.startsWith('vmess://')) {
          try {
            let b64 = trimmedText.replace('vmess://', '').trim();
            while (b64.length % 4 !== 0) { b64 += '='; }
            const decoded = atob(b64);
            const parsed = JSON.parse(decoded);
            if (parsed.ps) {
              tagPart = parsed.ps;
            }
          } catch {}
        } else {
          const parts = trimmedText.split('#');
          if (parts.length > 1) {
            tagPart = decodeURIComponent(parts[1].trim());
          }
        }
        if (tagPart) {
          setImportName(tagPart);
        }
      }
    }
  };

  const handleImportProfile = async (e: React.FormEvent) => {
    e.preventDefault();
    setImportError(null); setImportSuccess(false); setIsImporting(true);
    if (!importName.trim()) { setImportError('Please enter a profile name.'); setIsImporting(false); return; }
    if (!importContent.trim()) { setImportError('Please paste config content.'); setIsImporting(false); return; }
    try {
      const imported = await invoke<Profile>('import_from_clipboard', { content: importContent, name: importName });
      await storeHelper.addProfile(imported);
      const reloaded = await storeHelper.getProfiles();
      setProfiles(reloaded); setSelectedProfileId(imported.id);
      setImportSuccess(true); setImportName(''); setImportContent('');
      pushSystemLog(`Profile "${imported.name}" imported! ${imported.nodeCount} nodes.`);
      setTimeout(() => setImportSuccess(false), 3000);
    } catch (err) { setImportError(String(err)); }
    finally { setIsImporting(false); }
  };

  const handleSaveSettings = async (updates: Partial<Settings>) => {
    if (updates.proxyMode !== undefined) {
      const modeName = updates.proxyMode === 'tun' ? 'Virtual TUN Interface' : 'System Proxy';
      const confirmed = await ask(
        `Are you sure you want to switch the routing mode to ${modeName}? This will restart your active proxy connections.`,
        { title: 'Confirm Routing Mode Change', kind: 'warning' }
      );
      if (!confirmed) {
        return;
      }
    }

    const next = { ...settings, ...updates };
    setSettings(next);
    await storeHelper.saveSettings(updates);
    if (updates.proxyMode !== undefined) {
      if (updates.proxyMode === 'tun' && isElevated) {
        try { await invoke('set_runas_admin', { enabled: true }); } catch { }
      } else if (updates.proxyMode === 'system') {
        try { await invoke('set_runas_admin', { enabled: false }); } catch { }
      }
    }
    if (updates.autostart !== undefined) {
      try { await invoke('set_autostart', { enabled: updates.autostart }); } catch { }
    }
  };

  // ── ROUTING HANDLERS ───────────────────────────────────────────────────────
  const handleSaveRoutingRule = async () => {
    if (!ruleForm.value.trim()) return;
    const updated = editingRule
      ? routingRules.map((r) => r.id === editingRule.id ? { ...ruleForm, id: editingRule.id } : r)
      : [...routingRules, { ...ruleForm, id: uid() }];
    setRoutingRules(updated);
    await storeHelper.saveRoutingRules(updated);
    setShowAddRule(false); setEditingRule(null);
    setRuleForm({ type: 'geoip', value: '', outbound: 'direct', invert: false, notes: '' });
  };

  const handleDeleteRoutingRule = async (id: string) => {
    const updated = routingRules.filter((r) => r.id !== id);
    setRoutingRules(updated);
    await storeHelper.saveRoutingRules(updated);
  };

  const handleSaveRuleSet = async () => {
    if (!ruleSetForm.tag.trim()) return;
    const updated = [...ruleSets, { ...ruleSetForm, id: uid() }];
    setRuleSets(updated);
    await storeHelper.saveRuleSets(updated);
    setShowAddRuleSet(false);
    setRuleSetForm({ tag: '', type: 'remote', format: 'binary', url: '', updateInterval: '1d' });
  };

  const handleDeleteRuleSet = async (id: string) => {
    const updated = ruleSets.filter((r) => r.id !== id);
    setRuleSets(updated);
    await storeHelper.saveRuleSets(updated);
  };

  const handleUpdateRuleSet = async (rs: RuleSet) => {
    setUpdatingRuleSet(rs.id);
    try {
      await invoke('update_rule_set', { ruleSetId: rs.id, url: rs.url });
      const updated = ruleSets.map((r) => r.id === rs.id ? { ...r, lastUpdated: Date.now() } : r);
      setRuleSets(updated);
      await storeHelper.saveRuleSets(updated);
    } catch (err) { pushSystemLog(`Failed to update rule set: ${err}`); }
    finally { setUpdatingRuleSet(null); }
  };

  // ── DNS HANDLERS ───────────────────────────────────────────────────────────
  const handleSaveDnsRule = async () => {
    if (!dnsRuleForm.value.trim()) return;
    const updated = editingDnsRule
      ? dnsRules.map((r) => r.id === editingDnsRule.id ? { ...dnsRuleForm, id: editingDnsRule.id } : r)
      : [...dnsRules, { ...dnsRuleForm, id: uid() }];
    setDnsRules(updated);
    await storeHelper.saveDnsRules(updated);
    setShowAddDnsRule(false); setEditingDnsRule(null);
    setDnsRuleForm({ type: 'geosite', value: '', server: 'direct', disableCache: false, invert: false });
  };

  const handleDeleteDnsRule = async (id: string) => {
    const updated = dnsRules.filter((r) => r.id !== id);
    setDnsRules(updated);
    await storeHelper.saveDnsRules(updated);
  };

  // ── NODE EDITOR ────────────────────────────────────────────────────────────
  const handleOpenEditor = (node: any) => {
    setEditSaveError(null); setEditorSection('basic');
    setEditTag(node.tag || ''); setEditAddress(node.server || '');
    setEditPort(node.server_port || 443); setEditProtocol(node.type || 'vless');
    setEditUuid(node.uuid || node.id || ''); setEditFlow(node.flow || '');
    setEditPassword(node.password || ''); setEditMethod(node.method || 'aes-256-gcm');
    setEditHy2Auth(node.password || node.auth || '');
    setEditHy2UpBw(node.up_mbps ? `${node.up_mbps} mbps` : '100 mbps');
    setEditHy2DownBw(node.down_mbps ? `${node.down_mbps} mbps` : '200 mbps');
    setEditHy2ObfsType(node.obfs?.type || ''); setEditHy2ObfsPassword(node.obfs?.password || '');
    setEditTuicUuid(node.uuid || ''); setEditTuicPassword(node.password || '');
    setEditTuicCongestion(node.congestion_control || 'bbr'); setEditTuicUdpMode(node.udp_relay_mode || 'native');
    setEditWgSecretKey(node.secret_key || node.private_key || '');
    const peer = node.peers?.[0] || {};
    setEditWgPeerPublicKey(peer.public_key || ''); setEditWgPreSharedKey(peer.pre_shared_key || '');
    setEditWgEndpoint(peer.server ? `${peer.server}:${peer.server_port || 51820}` : '');
    setEditWgAllowedIps((peer.allowed_ips || ['0.0.0.0/0', '::/0']).join(', '));
    setEditWgReserved((node.reserved || []).join(','));
    setEditWgMtu(node.mtu || 1280);
    setEditSshUser(node.user || 'root'); setEditSshPassword(node.password || '');
    setEditSshPrivateKey(node.private_key || ''); setEditSshHostKey(node.host_key || '');
    setEditSocksUser(node.username || ''); setEditSocksPassword(node.password || '');
    setEditSocksVersion(node.version || 5); setEditSocksUdpOverTcp(node.udp_over_tcp || false);
    setEditHttpUser(node.username || ''); setEditHttpPassword(node.password || ''); setEditHttpTls(!!node.tls?.enabled);
    const t = node.transport?.type || node.network || 'tcp';
    setEditNetwork(['ws','grpc','quic','http','httpupgrade'].includes(t) ? t : 'tcp');
    setEditPath(node.transport?.path || ''); setEditHost(node.transport?.headers?.Host || '');
    setEditServiceName(node.transport?.service_name || '');
    setEditTlsEnabled(!!node.tls?.enabled); setEditAllowInsecure(!!node.tls?.insecure);
    setEditServerName(node.tls?.server_name || ''); setEditAlpn(node.tls?.alpn?.join(', ') || '');
    setEditRealityEnabled(!!node.tls?.reality?.enabled);
    setEditFingerprint(node.tls?.utls?.fingerprint || 'chrome');
    setEditPublicKey(node.tls?.reality?.public_key || ''); setEditShortId(node.tls?.reality?.short_id || '');
    setIsEditingConfig(true);
  };

  const handleSaveEditor = async () => {
    if (!selectedProfileId) return;
    setEditSaveError(null); setIsSavingConfig(true);
    if (!editTag.trim()) { setEditSaveError('Tag name is required.'); setIsSavingConfig(false); return; }
    try {
      const outbound: any = { type: editProtocol, tag: editTag.trim() };
      if (['vless','vmess','trojan','shadowsocks','hysteria2','tuic','socks','http'].includes(editProtocol)) {
        outbound.server = editAddress.trim(); outbound.server_port = Number(editPort) || 443;
      }
      if (editProtocol === 'vless' || editProtocol === 'vmess') {
        if (!editUuid.trim()) { setEditSaveError('UUID required.'); setIsSavingConfig(false); return; }
        outbound.uuid = editUuid.trim();
        if (editFlow && editProtocol === 'vless') outbound.flow = editFlow;
      } else if (editProtocol === 'trojan') {
        outbound.password = editPassword.trim();
      } else if (editProtocol === 'shadowsocks') {
        outbound.method = editMethod || 'aes-256-gcm'; outbound.password = editPassword.trim();
      } else if (editProtocol === 'hysteria2') {
        outbound.password = editHy2Auth.trim();
        if (editHy2UpBw) outbound.up_mbps = parseFloat(editHy2UpBw) || undefined;
        if (editHy2DownBw) outbound.down_mbps = parseFloat(editHy2DownBw) || undefined;
        if (editHy2ObfsType) outbound.obfs = { type: editHy2ObfsType, password: editHy2ObfsPassword };
      } else if (editProtocol === 'tuic') {
        outbound.uuid = editTuicUuid.trim(); outbound.password = editTuicPassword.trim();
        outbound.congestion_control = editTuicCongestion; outbound.udp_relay_mode = editTuicUdpMode;
      } else if (editProtocol === 'wireguard') {
        outbound.secret_key = editWgSecretKey.trim();
        const [epHost, epPort] = editWgEndpoint.split(':');
        outbound.peers = [{ public_key: editWgPeerPublicKey.trim(), pre_shared_key: editWgPreSharedKey.trim() || undefined, server: epHost, server_port: parseInt(epPort) || 51820, allowed_ips: editWgAllowedIps.split(',').map((s) => s.trim()).filter(Boolean) }];
        if (editWgReserved.trim()) outbound.reserved = editWgReserved.split(',').map((s) => parseInt(s.trim())).filter((n) => !isNaN(n));
        outbound.mtu = editWgMtu;
      } else if (editProtocol === 'ssh') {
        outbound.user = editSshUser.trim();
        if (editSshPassword) outbound.password = editSshPassword;
        if (editSshPrivateKey) outbound.private_key = editSshPrivateKey;
        if (editSshHostKey) outbound.host_key = [editSshHostKey];
      } else if (editProtocol === 'socks') {
        outbound.version = editSocksVersion;
        if (editSocksUser) { outbound.username = editSocksUser; outbound.password = editSocksPassword; }
        outbound.udp_over_tcp = editSocksUdpOverTcp;
      } else if (editProtocol === 'http') {
        if (editHttpUser) { outbound.username = editHttpUser; outbound.password = editHttpPassword; }
        if (editHttpTls) outbound.tls = { enabled: true };
      }
      if (!['wireguard','ssh','socks','http','hysteria2','tuic'].includes(editProtocol)) {
        if (editTlsEnabled) {
          const tls: any = { enabled: true };
          if (editServerName.trim()) tls.server_name = editServerName.trim();
          if (editAllowInsecure) tls.insecure = true;
          if (editAlpn.trim()) tls.alpn = editAlpn.split(',').map((s) => s.trim()).filter(Boolean);
          if (editFingerprint) tls.utls = { enabled: true, fingerprint: editFingerprint };
          if (editRealityEnabled) {
            if (!editPublicKey.trim()) { setEditSaveError('Reality Public Key required.'); setIsSavingConfig(false); return; }
            tls.reality = { enabled: true, public_key: editPublicKey.trim(), short_id: editShortId.trim() || undefined };
          }
          outbound.tls = tls;
        }
        if (editNetwork && editNetwork !== 'tcp') {
          const transport: any = { type: editNetwork };
          if (editPath.trim()) transport.path = editPath.trim();
          if (editHost.trim()) transport.headers = { Host: editHost.trim() };
          if (editServiceName.trim()) transport.service_name = editServiceName.trim();
          outbound.transport = transport;
        }
      }
      await invoke('update_profile_config', { profileId: selectedProfileId, newOutbound: outbound });
      const outbounds = await invoke<any[]>('get_profile_outbounds', { profileId: selectedProfileId });
      setProfileOutbounds(outbounds || []);
      const active = await invoke<any>('get_profile_outbound', { profileId: selectedProfileId });
      if (active) setSelectedOutboundTag(active.tag);
      pushSystemLog(`Node "${outbound.tag}" updated and validated.`);
      setIsEditingConfig(false);
    } catch (err) { setEditSaveError(String(err)); }
    finally { setIsSavingConfig(false); }
  };


  // ── RENDER ─────────────────────────────────────────────────────────────────
  return (
    <div className="app-root">
      <NavRail
        activeTab={activeTab}
        onTabChange={setActiveTab}
        isConnected={isConnected}
        singboxVersion={singboxVersion}
      />

      <div className="view-host">
        {activeTab === 'dashboard' && (
          <DashboardView
            isConnected={isConnected}
            activeProfileId={activeProfileId}
            uptime={uptime}
            httpPort={httpPort}
            socksPort={socksPort}
            mixedPort={mixedPort}
            isElevated={isElevated}
            singboxVersion={singboxVersion}
            appVersion={appVersion}
            uploadBytes={uploadBytes}
            downloadBytes={downloadBytes}
            uploadSpeed={uploadSpeed}
            downloadSpeed={downloadSpeed}
            activeConnections={activeConnections}
            speedHistory={speedHistory}
            profiles={profiles}
            settings={settings}
            selectedProfileId={selectedProfileId}
            profileOutbounds={profileOutbounds}
            selectedOutboundTag={selectedOutboundTag}
            onToggleConnect={handleToggleConnect}
            onRequestElevation={handleRequestElevation}
            onSelectOutbound={handleSelectOutbound}
            onOpenEditor={handleOpenEditor}
          />
        )}

        {activeTab === 'profiles' && (
          <ProfilesView
            profiles={profiles}
            selectedProfileId={selectedProfileId}
            activeProfileId={activeProfileId}
            isConnected={isConnected}
            importName={importName}
            importContent={importContent}
            importError={importError}
            importSuccess={importSuccess}
            isImporting={isImporting}
            onSelectProfile={setSelectedProfileId}
            onDeleteProfile={handleDeleteProfile}
            onSetImportName={setImportName}
            onSetImportContent={handleImportContentChange}
            onPasteClipboard={handlePasteClipboard}
            onImportProfile={handleImportProfile}
          />
        )}

        {activeTab === 'routing' && (
          <RoutingView
            routingRules={routingRules}
            ruleSets={ruleSets}
            showAddRule={showAddRule}
            editingRule={editingRule}
            ruleForm={ruleForm}
            showAddRuleSet={showAddRuleSet}
            ruleSetForm={ruleSetForm}
            updatingRuleSet={updatingRuleSet}
            settings={settings}
            onSetShowAddRule={setShowAddRule}
            onSetEditingRule={setEditingRule}
            onSetRuleForm={setRuleForm}
            onSetShowAddRuleSet={setShowAddRuleSet}
            onSetRuleSetForm={setRuleSetForm}
            onSaveRule={handleSaveRoutingRule}
            onDeleteRule={handleDeleteRoutingRule}
            onSaveRuleSet={handleSaveRuleSet}
            onDeleteRuleSet={handleDeleteRuleSet}
            onUpdateRuleSet={handleUpdateRuleSet}
            onSaveSettings={handleSaveSettings}
          />
        )}

        {activeTab === 'dns' && (
          <DnsView
            dnsRules={dnsRules}
            showAddDnsRule={showAddDnsRule}
            editingDnsRule={editingDnsRule}
            dnsRuleForm={dnsRuleForm}
            settings={settings}
            onSetShowAddDnsRule={setShowAddDnsRule}
            onSetEditingDnsRule={setEditingDnsRule}
            onSetDnsRuleForm={setDnsRuleForm}
            onSaveDnsRule={handleSaveDnsRule}
            onDeleteDnsRule={handleDeleteDnsRule}
            onSaveSettings={handleSaveSettings}
          />
        )}

        {activeTab === 'logs' && (
          <LogsView
            logs={logs}
            autoScroll={autoScrollLogs}
            onSetAutoScroll={setAutoScrollLogs}
            onClearLogs={() => setLogs([])}
            onCopyLogs={() => { navigator.clipboard.writeText(logs.map((l) => l.text).join('\n')); pushSystemLog('Copied all logs to clipboard.'); }}
          />
        )}

        {activeTab === 'settings' && (
          <SettingsView
            settings={settings}
            settingsTab={settingsTab}
            conflictingPorts={conflictingPorts}
            isElevated={isElevated}
            singboxVersion={singboxVersion}
            appVersion={appVersion}
            httpPort={httpPort}
            socksPort={socksPort}
            mixedPort={mixedPort}
            isConnected={isConnected}
            onSetSettingsTab={setSettingsTab}
            onSaveSettings={handleSaveSettings}
            onSetHttpPort={setHttpPort}
            onSetSocksPort={setSocksPort}
            onSetMixedPort={setMixedPort}
          />
        )}
      </div>

      {/* Node Editor Drawer */}
      <NodeEditor
        isOpen={isEditingConfig}
        onClose={() => setIsEditingConfig(false)}
        onSave={handleSaveEditor}
        isSaving={isSavingConfig}
        saveError={editSaveError}
        section={editorSection}
        onSectionChange={setEditorSection}

        editTag={editTag} setEditTag={setEditTag}
        editAddress={editAddress} setEditAddress={setEditAddress}
        editPort={editPort} setEditPort={setEditPort}
        editProtocol={editProtocol} setEditProtocol={setEditProtocol}
        editUuid={editUuid} setEditUuid={setEditUuid}
        editFlow={editFlow} setEditFlow={setEditFlow}
        editPassword={editPassword} setEditPassword={setEditPassword}
        editMethod={editMethod} setEditMethod={setEditMethod}
        editNetwork={editNetwork} setEditNetwork={setEditNetwork}
        editHeaderType={editHeaderType} setEditHeaderType={setEditHeaderType}
        editPath={editPath} setEditPath={setEditPath}
        editHost={editHost} setEditHost={setEditHost}
        editServiceName={editServiceName} setEditServiceName={setEditServiceName}
        editTlsEnabled={editTlsEnabled} setEditTlsEnabled={setEditTlsEnabled}
        editAllowInsecure={editAllowInsecure} setEditAllowInsecure={setEditAllowInsecure}
        editServerName={editServerName} setEditServerName={setEditServerName}
        editAlpn={editAlpn} setEditAlpn={setEditAlpn}
        editRealityEnabled={editRealityEnabled} setEditRealityEnabled={setEditRealityEnabled}
        editFingerprint={editFingerprint} setEditFingerprint={setEditFingerprint}
        editPublicKey={editPublicKey} setEditPublicKey={setEditPublicKey}
        editShortId={editShortId} setEditShortId={setEditShortId}
        editHy2Auth={editHy2Auth} setEditHy2Auth={setEditHy2Auth}
        editHy2UpBw={editHy2UpBw} setEditHy2UpBw={setEditHy2UpBw}
        editHy2DownBw={editHy2DownBw} setEditHy2DownBw={setEditHy2DownBw}
        editHy2ObfsType={editHy2ObfsType} setEditHy2ObfsType={setEditHy2ObfsType}
        editHy2ObfsPassword={editHy2ObfsPassword} setEditHy2ObfsPassword={setEditHy2ObfsPassword}
        editTuicUuid={editTuicUuid} setEditTuicUuid={setEditTuicUuid}
        editTuicPassword={editTuicPassword} setEditTuicPassword={setEditTuicPassword}
        editTuicCongestion={editTuicCongestion} setEditTuicCongestion={setEditTuicCongestion}
        editTuicUdpMode={editTuicUdpMode} setEditTuicUdpMode={setEditTuicUdpMode}
        editWgSecretKey={editWgSecretKey} setEditWgSecretKey={setEditWgSecretKey}
        editWgPeerPublicKey={editWgPeerPublicKey} setEditWgPeerPublicKey={setEditWgPeerPublicKey}
        editWgPreSharedKey={editWgPreSharedKey} setEditWgPreSharedKey={setEditWgPreSharedKey}
        editWgEndpoint={editWgEndpoint} setEditWgEndpoint={setEditWgEndpoint}
        editWgAllowedIps={editWgAllowedIps} setEditWgAllowedIps={setEditWgAllowedIps}
        editWgReserved={editWgReserved} setEditWgReserved={setEditWgReserved}
        editWgMtu={editWgMtu} setEditWgMtu={setEditWgMtu}
        editSshUser={editSshUser} setEditSshUser={setEditSshUser}
        editSshPassword={editSshPassword} setEditSshPassword={setEditSshPassword}
        editSshPrivateKey={editSshPrivateKey} setEditSshPrivateKey={setEditSshPrivateKey}
        editSshHostKey={editSshHostKey} setEditSshHostKey={setEditSshHostKey}
        editSocksUser={editSocksUser} setEditSocksUser={setEditSocksUser}
        editSocksPassword={editSocksPassword} setEditSocksPassword={setEditSocksPassword}
        editSocksVersion={editSocksVersion} setEditSocksVersion={setEditSocksVersion}
        editSocksUdpOverTcp={editSocksUdpOverTcp} setEditSocksUdpOverTcp={setEditSocksUdpOverTcp}
        editHttpUser={editHttpUser} setEditHttpUser={setEditHttpUser}
        editHttpPassword={editHttpPassword} setEditHttpPassword={setEditHttpPassword}
        editHttpTls={editHttpTls} setEditHttpTls={setEditHttpTls}
      />
    </div>
  );
}
