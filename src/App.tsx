// @ts-nocheck
import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { readText } from '@tauri-apps/plugin-clipboard-manager';
import {
  Activity, Layers, Terminal, Settings as SettingsIcon, Power,
  UploadCloud, DownloadCloud, Clock, Cpu, Globe, Plus, Trash2,
  Check, RefreshCw, FolderOpen, Clipboard, Shield, ShieldAlert,
  Server, Zap, Route, Dna, Database, Filter, Eye, Network,
  Code2, ShieldCheck, Ghost, GitFork, Wifi, AlertTriangle,
  ChevronRight, ChevronDown, X, Info, Download, Key, Lock,
  Layers2, Radio, Hash, FileCode2, ArrowUpDown, ToggleLeft,
  CheckCircle2, XCircle, Copy, ExternalLink, Edit3,
} from 'lucide-react';
import { TrafficChart } from './components/TrafficChart';
import { storeHelper } from './utils/store';
import type {
  Profile, Settings, RoutingRule, RuleSet, DnsRule,
  RoutingRuleType, OutboundAction, DnsRuleType, DnsStrategy, DnsMode
} from './utils/store';

// ── FORMATTERS ────────────────────────────────────────────────────────────────
const formatBytes = (bytes: number): string => {
  if (bytes === 0) return '0 B';
  const k = 1024, sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
};
const formatSpeed = (bps: number): string => {
  if (bps === 0) return '0 B/s';
  const k = 1024, sizes = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
  const i = Math.floor(Math.log(bps) / Math.log(k));
  return parseFloat((bps / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
};
const formatUptime = (seconds: number): string => {
  const h = Math.floor(seconds / 3600).toString().padStart(2, '0');
  const m = Math.floor((seconds % 3600) / 60).toString().padStart(2, '0');
  const s = (seconds % 60).toString().padStart(2, '0');
  return `${h}h ${m}m ${s}s`;
};
const uid = () => Math.random().toString(36).slice(2, 10);

const OUTBOUND_COLORS: Record<OutboundAction, string> = {
  proxy: 'var(--accent-cyan)',
  direct: 'var(--status-connected)',
  block: 'var(--status-error)',
  dns: 'var(--accent-purple)',
};

const PROTOCOL_LABELS: Record<string, string> = {
  vless: 'VLESS', vmess: 'VMess', trojan: 'Trojan',
  shadowsocks: 'Shadowsocks', hysteria2: 'Hysteria2', tuic: 'TUIC',
  wireguard: 'WireGuard', ssh: 'SSH', socks: 'SOCKS5', http: 'HTTP',
};

export default function App() {
  // ── NAVIGATION ──────────────────────────────────────────────────────────────
  type TabId = 'dashboard' | 'profiles' | 'routing' | 'dns' | 'logs' | 'settings';
  const [activeTab, setActiveTab] = useState<TabId>('dashboard');

  // ── APP CORE STATE ───────────────────────────────────────────────────────────
  const [isConnected, setIsConnected] = useState(false);
  const [activeProfileId, setActiveProfileId] = useState<string | null>(null);
  const [uptime, setUptime] = useState(0);
  const [httpPort, setHttpPort] = useState(7890);
  const [socksPort, setSocksPort] = useState(7891);
  const [mixedPort, setMixedPort] = useState(7892);
  const [isElevated, setIsElevated] = useState(false);
  const [singboxVersion, setSingboxVersion] = useState('Unknown');
  const [appVersion, setAppVersion] = useState('0.1.0');

  // ── TRAFFIC ──────────────────────────────────────────────────────────────────
  const [uploadBytes, setUploadBytes] = useState(0);
  const [downloadBytes, setDownloadBytes] = useState(0);
  const [uploadSpeed, setUploadSpeed] = useState(0);
  const [downloadSpeed, setDownloadSpeed] = useState(0);
  const [activeConnections, setActiveConnections] = useState(0);
  const [speedHistory, setSpeedHistory] = useState<{ up: number; down: number }[]>([]);

  // ── PROFILES ─────────────────────────────────────────────────────────────────
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [selectedProfileId, setSelectedProfileId] = useState<string | null>(null);
  const [profileOutbounds, setProfileOutbounds] = useState<any[]>([]);
  const [selectedOutboundTag, setSelectedOutboundTag] = useState<string | null>(null);
  const [importTab, setImportTab] = useState<'url' | 'file' | 'clipboard'>('url');
  const [importName, setImportName] = useState('');
  const [importUrl, setImportUrl] = useState('');
  const [importContent, setImportContent] = useState('');
  const [importFilePath, setImportFilePath] = useState('');
  const [importError, setImportError] = useState<string | null>(null);
  const [importSuccess, setImportSuccess] = useState(false);
  const [isImporting, setIsImporting] = useState(false);

  // ── SETTINGS ─────────────────────────────────────────────────────────────────
  const [settings, setSettings] = useState<Settings>({
    proxyMode: 'system', closeToTray: true, autostart: false,
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
    dnsAddress: '', sniHost: 'aka.ms',
  });
  const [conflictingPorts, setConflictingPorts] = useState<number[]>([]);
  const [settingsTab, setSettingsTab] = useState<'general' | 'tun' | 'sniff' | 'mux' | 'api'>('general');

  // ── ROUTING ──────────────────────────────────────────────────────────────────
  const [routingRules, setRoutingRules] = useState<RoutingRule[]>([]);
  const [ruleSets, setRuleSets] = useState<RuleSet[]>([]);
  const [showAddRule, setShowAddRule] = useState(false);
  const [editingRule, setEditingRule] = useState<RoutingRule | null>(null);
  const [ruleForm, setRuleForm] = useState<Omit<RoutingRule, 'id'>>({
    type: 'geoip', value: '', outbound: 'direct', invert: false, notes: '',
  });
  const [showAddRuleSet, setShowAddRuleSet] = useState(false);
  const [ruleSetForm, setRuleSetForm] = useState<Omit<RuleSet, 'id'>>({
    tag: '', type: 'remote', format: 'binary', url: '', updateInterval: '1d',
  });
  const [updatingRuleSet, setUpdatingRuleSet] = useState<string | null>(null);

  // ── DNS ───────────────────────────────────────────────────────────────────────
  const [dnsRules, setDnsRules] = useState<DnsRule[]>([]);
  const [showAddDnsRule, setShowAddDnsRule] = useState(false);
  const [editingDnsRule, setEditingDnsRule] = useState<DnsRule | null>(null);
  const [dnsRuleForm, setDnsRuleForm] = useState<Omit<DnsRule, 'id'>>({
    type: 'geosite', value: '', server: 'direct', disableCache: false, invert: false,
  });

  // ── LOGS ──────────────────────────────────────────────────────────────────────
  const [logs, setLogs] = useState<{ type: 'info'|'warn'|'error'|'system'; text: string }[]>([]);
  const [autoScrollLogs, setAutoScrollLogs] = useState(true);
  const consoleEndRef = useRef<HTMLDivElement | null>(null);

  // ── NODE EDITOR ───────────────────────────────────────────────────────────────
  const [isEditingConfig, setIsEditingConfig] = useState(false);
  const [editTag, setEditTag] = useState('');
  const [editAddress, setEditAddress] = useState('');
  const [editPort, setEditPort] = useState(443);
  const [editProtocol, setEditProtocol] = useState('vless');
  // VLESS/VMess
  const [editUuid, setEditUuid] = useState('');
  const [editFlow, setEditFlow] = useState('');
  // Trojan/SS/common
  const [editPassword, setEditPassword] = useState('');
  const [editMethod, setEditMethod] = useState('aes-256-gcm');
  // Transport
  const [editNetwork, setEditNetwork] = useState<'tcp'|'ws'|'grpc'|'quic'|'http'|'httpupgrade'>('tcp');
  const [editHeaderType, setEditHeaderType] = useState('');
  const [editPath, setEditPath] = useState('');
  const [editHost, setEditHost] = useState('');
  const [editServiceName, setEditServiceName] = useState('');
  // TLS
  const [editTlsEnabled, setEditTlsEnabled] = useState(false);
  const [editAllowInsecure, setEditAllowInsecure] = useState(false);
  const [editServerName, setEditServerName] = useState('');
  const [editAlpn, setEditAlpn] = useState('');
  const [editRealityEnabled, setEditRealityEnabled] = useState(false);
  const [editFingerprint, setEditFingerprint] = useState('chrome');
  const [editPublicKey, setEditPublicKey] = useState('');
  const [editShortId, setEditShortId] = useState('');
  // Hysteria2
  const [editHy2Auth, setEditHy2Auth] = useState('');
  const [editHy2UpBw, setEditHy2UpBw] = useState('100 mbps');
  const [editHy2DownBw, setEditHy2DownBw] = useState('200 mbps');
  const [editHy2ObfsType, setEditHy2ObfsType] = useState('');
  const [editHy2ObfsPassword, setEditHy2ObfsPassword] = useState('');
  // TUIC
  const [editTuicUuid, setEditTuicUuid] = useState('');
  const [editTuicPassword, setEditTuicPassword] = useState('');
  const [editTuicCongestion, setEditTuicCongestion] = useState('bbr');
  const [editTuicUdpMode, setEditTuicUdpMode] = useState('native');
  // WireGuard
  const [editWgSecretKey, setEditWgSecretKey] = useState('');
  const [editWgPeerPublicKey, setEditWgPeerPublicKey] = useState('');
  const [editWgPreSharedKey, setEditWgPreSharedKey] = useState('');
  const [editWgEndpoint, setEditWgEndpoint] = useState('');
  const [editWgAllowedIps, setEditWgAllowedIps] = useState('0.0.0.0/0, ::/0');
  const [editWgReserved, setEditWgReserved] = useState('');
  const [editWgMtu, setEditWgMtu] = useState(1280);
  // SSH
  const [editSshUser, setEditSshUser] = useState('root');
  const [editSshPassword, setEditSshPassword] = useState('');
  const [editSshPrivateKey, setEditSshPrivateKey] = useState('');
  const [editSshHostKey, setEditSshHostKey] = useState('');
  // SOCKS5 / HTTP outbound
  const [editSocksUser, setEditSocksUser] = useState('');
  const [editSocksPassword, setEditSocksPassword] = useState('');
  const [editSocksVersion, setEditSocksVersion] = useState(5);
  const [editSocksUdpOverTcp, setEditSocksUdpOverTcp] = useState(false);
  const [editHttpUser, setEditHttpUser] = useState('');
  const [editHttpPassword, setEditHttpPassword] = useState('');
  const [editHttpTls, setEditHttpTls] = useState(false);
  // Editor status
  const [editSaveError, setEditSaveError] = useState<string | null>(null);
  const [isSavingConfig, setIsSavingConfig] = useState(false);
  const [editorSection, setEditorSection] = useState<'basic'|'transport'|'tls'>('basic');

  // ─────────────────────────────────────────────────────────────────────────────
  // INITIALIZATION
  // ─────────────────────────────────────────────────────────────────────────────
  useEffect(() => {
    (async () => {
      try {
        setIsElevated(await invoke<boolean>('check_tun_support'));
        setAppVersion(await invoke<string>('get_app_version'));
        setSingboxVersion(await invoke<string>('get_singbox_version'));
      } catch { /* no-op */ }

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

    pushSystemLog('TunX Core Engine initialized.');
    pushSystemLog('Ready to establish proxy tunnels.');

    const unlistenLog = listen<string>('sing-box-log', (e) => {
      const line = e.payload.trim();
      if (!line) return;
      let type: 'info'|'warn'|'error'|'system' = 'info';
      if (line.toLowerCase().includes('warn')) type = 'warn';
      else if (line.toLowerCase().includes('err') || line.toLowerCase().includes('fatal')) type = 'error';
      setLogs((prev) => [...prev, { type, text: line }]);
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
            let type: 'info'|'warn'|'error'|'system' = 'info';
            if (line.toLowerCase().includes('warn')) type = 'warn';
            else if (line.toLowerCase().includes('err')) type = 'error';
            return { type, text: line };
          })]);
        }
      } catch { /* no-op */ }
    })();

    return () => {
      unlistenLog.then((f) => f());
      unlistenTerm.then((f) => f());
    };
  }, []);

  // Port conflict checker
  useEffect(() => {
    (async () => {
      const ports = [httpPort, socksPort, mixedPort];
      const conflicts: number[] = [];
      for (const p of ports) {
        try {
          const res = await invoke<number | null>('check_port_conflict', { ports: [p] });
          if (res) conflicts.push(p);
        } catch { /* no-op */ }
      }
      setConflictingPorts(conflicts);
    })();
  }, [activeTab, httpPort, socksPort, mixedPort]);

  // Poll timer
  useEffect(() => {
    const poll = setInterval(async () => {
      try {
        const status = await invoke<{
          status: string; activeProfileId: string | null;
          httpPort: number; socksPort: number; mixedPort: number;
          tunEnabled: boolean; uptime: number;
        }>('get_proxy_status');
        const active = status.status === 'connected';
        setIsConnected(active);
        setActiveProfileId(status.activeProfileId);
        setUptime(status.uptime);
        if (active) {
          const stats = await invoke<{
            uploadBytes: number; downloadBytes: number;
            uploadSpeed: number; downloadSpeed: number; activeConnections: number;
          }>('get_traffic_stats');
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
      } catch { /* no-op */ }
    }, 1000);
    return () => clearInterval(poll);
  }, []);

  // Auto-scroll logs
  useEffect(() => {
    if (autoScrollLogs && consoleEndRef.current)
      consoleEndRef.current.scrollIntoView({ behavior: 'smooth' });
  }, [logs, autoScrollLogs]);

  // Load outbounds when profile changes
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

  // ── LOG HELPER ────────────────────────────────────────────────────────────────
  const pushSystemLog = (text: string) =>
    setLogs((prev) => [...prev, { type: 'system', text: `[System] ${text}` }]);

  // ── ACTIONS ───────────────────────────────────────────────────────────────────
  const handleToggleConnect = async () => {
    if (isConnected) {
      pushSystemLog('Stopping TunX tunnel proxy services...');
      try {
        const result = await invoke<string>('toggle_proxy', { start: false, profileId: '' });
        if (result === 'stopped') {
          setIsConnected(false); setActiveProfileId(null);
          pushSystemLog('Proxy tunnels stopped. System proxy reverted.');
        }
      } catch (e) { pushSystemLog(`Error stopping proxy: ${e}`); }
    } else {
      if (!selectedProfileId) {
        pushSystemLog('Error: Select or import a profile first.');
        setActiveTab('profiles'); return;
      }
      try {
        const conflict = await invoke<number | null>('check_port_conflict', {
          ports: [httpPort, socksPort, mixedPort],
        });
        if (conflict) {
          pushSystemLog(`Error: Port ${conflict} is in use. Change ports in Settings.`);
          setActiveTab('settings'); return;
        }
      } catch { /* skip */ }
      const target = profiles.find((p) => p.id === selectedProfileId);
      pushSystemLog(`Booting TunX Core using profile "${target?.name || 'Default'}"...`);
      try {
        const result = await invoke<string>('toggle_proxy', { start: true, profileId: selectedProfileId });
        if (result === 'started') {
          setIsConnected(true); setActiveProfileId(selectedProfileId);
          pushSystemLog(`sing-box established on ports HTTP:${httpPort}, Mixed:${mixedPort}.`);
        }
      } catch (e) { pushSystemLog(`Startup error: ${e}`); }
    }
  };

  const handleRequestElevation = async () => {
    pushSystemLog('Launching Administrator elevation (UAC)...');
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
      pushSystemLog('Profile deleted.');
      if (selectedProfileId === id)
        setSelectedProfileId(remaining.length > 0 ? remaining[0].id : null);
    } catch (err) { pushSystemLog(`Failed to delete profile: ${err}`); }
  };

  const handleSelectOutbound = async (node: any) => {
    if (!selectedProfileId) return;
    try {
      pushSystemLog(`Switching route to node "${node.tag}"...`);
      await invoke('update_profile_config', { profileId: selectedProfileId, newOutbound: node });
      setSelectedOutboundTag(node.tag);
      pushSystemLog(`Node switched to "${node.tag}". Config validated.`);
    } catch (err) { pushSystemLog(`Failed to switch node: ${err}`); }
  };

  const handlePickFile = async () => {
    try {
      const selected = await open({ multiple: false, filters: [{ name: 'Configs', extensions: ['json','yaml','yml','txt'] }] });
      if (selected) {
        setImportFilePath(selected as string);
        if (!importName) setImportName(((selected as string).split(/[\\/]/).pop() || '').replace(/\.[^/.]+$/, ''));
      }
    } catch { /* no-op */ }
  };

  const handlePasteClipboard = async () => {
    try {
      const text = await readText();
      if (text) { setImportContent(text); pushSystemLog('Pasted config from clipboard.'); }
      else setImportError('Clipboard is empty.');
    } catch { setImportError('Failed to read clipboard.'); }
  };

  const handleImportProfile = async (e: React.FormEvent) => {
    e.preventDefault();
    setImportError(null); setImportSuccess(false); setIsImporting(true);
    if (!importName.trim()) { setImportError('Please enter a profile name.'); setIsImporting(false); return; }
    try {
      let imported: Profile;
      if (importTab === 'url') {
        if (!importUrl.trim()) { setImportError('Please provide a subscription URL.'); setIsImporting(false); return; }
        pushSystemLog(`Downloading subscription: ${importUrl}`);
        imported = await invoke<Profile>('import_subscription', { url: importUrl, name: importName });
      } else if (importTab === 'file') {
        if (!importFilePath.trim()) { setImportError('Please select a config file.'); setIsImporting(false); return; }
        imported = await invoke<Profile>('import_file', { filePath: importFilePath, name: importName });
      } else {
        if (!importContent.trim()) { setImportError('Please paste config content.'); setIsImporting(false); return; }
        imported = await invoke<Profile>('import_from_clipboard', { content: importContent, name: importName });
      }
      await storeHelper.addProfile(imported);
      const reloaded = await storeHelper.getProfiles();
      setProfiles(reloaded); setSelectedProfileId(imported.id);
      setImportSuccess(true); setImportName(''); setImportUrl(''); setImportContent(''); setImportFilePath('');
      pushSystemLog(`Profile "${imported.name}" imported! ${imported.nodeCount} nodes.`);
      setTimeout(() => setImportSuccess(false), 3000);
    } catch (err) { setImportError(String(err)); pushSystemLog(`Import failed: ${err}`); }
    finally { setIsImporting(false); }
  };

  const handleSaveSettings = async (updates: Partial<Settings>) => {
    const next = { ...settings, ...updates };
    setSettings(next);
    await storeHelper.saveSettings(updates);
    if (updates.autostart !== undefined) {
      try {
        await invoke('set_autostart', { enabled: updates.autostart });
        pushSystemLog(`Autostart ${updates.autostart ? 'enabled' : 'disabled'}.`);
      } catch { /* no-op */ }
    }
  };

  // ── ROUTING HANDLERS ──────────────────────────────────────────────────────────
  const handleSaveRoutingRule = async () => {
    if (!ruleForm.value.trim()) return;
    let updatedRules: RoutingRule[];
    if (editingRule) {
      updatedRules = routingRules.map((r) =>
        r.id === editingRule.id ? { ...ruleForm, id: editingRule.id } : r
      );
    } else {
      updatedRules = [...routingRules, { ...ruleForm, id: uid() }];
    }
    setRoutingRules(updatedRules);
    await storeHelper.saveRoutingRules(updatedRules);
    setShowAddRule(false); setEditingRule(null);
    setRuleForm({ type: 'geoip', value: '', outbound: 'direct', invert: false, notes: '' });
    pushSystemLog(`Routing rule ${editingRule ? 'updated' : 'added'}: ${ruleForm.type} → ${ruleForm.outbound}`);
  };

  const handleDeleteRoutingRule = async (id: string) => {
    const updated = routingRules.filter((r) => r.id !== id);
    setRoutingRules(updated);
    await storeHelper.saveRoutingRules(updated);
    pushSystemLog('Routing rule removed.');
  };

  const handleSaveRuleSet = async () => {
    if (!ruleSetForm.tag.trim()) return;
    const updated = [...ruleSets, { ...ruleSetForm, id: uid() }];
    setRuleSets(updated);
    await storeHelper.saveRuleSets(updated);
    setShowAddRuleSet(false);
    setRuleSetForm({ tag: '', type: 'remote', format: 'binary', url: '', updateInterval: '1d' });
    pushSystemLog(`Rule set "${ruleSetForm.tag}" added.`);
  };

  const handleDeleteRuleSet = async (id: string) => {
    const updated = ruleSets.filter((r) => r.id !== id);
    setRuleSets(updated); await storeHelper.saveRuleSets(updated);
    pushSystemLog('Rule set removed.');
  };

  const handleUpdateRuleSet = async (rs: RuleSet) => {
    setUpdatingRuleSet(rs.id);
    pushSystemLog(`Updating rule set "${rs.tag}" from remote...`);
    try {
      await invoke('update_rule_set', { ruleSetId: rs.id, url: rs.url });
      const updated = ruleSets.map((r) => r.id === rs.id ? { ...r, lastUpdated: Date.now() } : r);
      setRuleSets(updated); await storeHelper.saveRuleSets(updated);
      pushSystemLog(`Rule set "${rs.tag}" updated successfully.`);
    } catch (err) {
      pushSystemLog(`Failed to update rule set "${rs.tag}": ${err}`);
    } finally { setUpdatingRuleSet(null); }
  };

  // ── DNS HANDLERS ──────────────────────────────────────────────────────────────
  const handleSaveDnsRule = async () => {
    if (!dnsRuleForm.value.trim()) return;
    let updated: DnsRule[];
    if (editingDnsRule) {
      updated = dnsRules.map((r) => r.id === editingDnsRule.id ? { ...dnsRuleForm, id: editingDnsRule.id } : r);
    } else {
      updated = [...dnsRules, { ...dnsRuleForm, id: uid() }];
    }
    setDnsRules(updated); await storeHelper.saveDnsRules(updated);
    setShowAddDnsRule(false); setEditingDnsRule(null);
    setDnsRuleForm({ type: 'geosite', value: '', server: 'direct', disableCache: false, invert: false });
    pushSystemLog(`DNS rule ${editingDnsRule ? 'updated' : 'added'}.`);
  };

  const handleDeleteDnsRule = async (id: string) => {
    const updated = dnsRules.filter((r) => r.id !== id);
    setDnsRules(updated); await storeHelper.saveDnsRules(updated);
    pushSystemLog('DNS rule removed.');
  };

  // ── NODE EDITOR ───────────────────────────────────────────────────────────────
  const handleOpenEditor = (node: any) => {
    setEditSaveError(null); setEditorSection('basic');
    setEditTag(node.tag || ''); setEditAddress(node.server || '');
    setEditPort(node.server_port || 443); setEditProtocol(node.type || 'vless');
    setEditUuid(node.uuid || node.id || ''); setEditFlow(node.flow || '');
    setEditPassword(node.password || ''); setEditMethod(node.method || 'aes-256-gcm');
    // Hysteria2
    setEditHy2Auth(node.password || node.auth || '');
    setEditHy2UpBw(node.up_mbps ? `${node.up_mbps} mbps` : '100 mbps');
    setEditHy2DownBw(node.down_mbps ? `${node.down_mbps} mbps` : '200 mbps');
    setEditHy2ObfsType(node.obfs?.type || '');
    setEditHy2ObfsPassword(node.obfs?.password || '');
    // TUIC
    setEditTuicUuid(node.uuid || ''); setEditTuicPassword(node.password || '');
    setEditTuicCongestion(node.congestion_control || 'bbr');
    setEditTuicUdpMode(node.udp_relay_mode || 'native');
    // WireGuard
    setEditWgSecretKey(node.secret_key || node.private_key || '');
    const peer = node.peers?.[0] || {};
    setEditWgPeerPublicKey(peer.public_key || ''); setEditWgPreSharedKey(peer.pre_shared_key || '');
    setEditWgEndpoint(peer.server ? `${peer.server}:${peer.server_port || 51820}` : '');
    setEditWgAllowedIps((peer.allowed_ips || ['0.0.0.0/0', '::/0']).join(', '));
    setEditWgReserved((node.reserved || []).join(','));
    setEditWgMtu(node.mtu || 1280);
    // SSH
    setEditSshUser(node.user || 'root'); setEditSshPassword(node.password || '');
    setEditSshPrivateKey(node.private_key || ''); setEditSshHostKey(node.host_key || '');
    // SOCKS5 / HTTP
    const user = node.username || ''; const pass = node.password || '';
    setEditSocksUser(user); setEditSocksPassword(pass);
    setEditSocksVersion(node.version || 5); setEditSocksUdpOverTcp(node.udp_over_tcp || false);
    setEditHttpUser(user); setEditHttpPassword(pass); setEditHttpTls(!!node.tls?.enabled);
    // Transport
    const t = node.transport?.type || node.network || 'tcp';
    setEditNetwork(t === 'ws' || t === 'grpc' || t === 'quic' || t === 'http' || t === 'httpupgrade' ? t : 'tcp');
    setEditHeaderType(node.transport?.headers?.type || '');
    setEditPath(node.transport?.path || ''); setEditHost(node.transport?.headers?.Host || node.transport?.host || '');
    setEditServiceName(node.transport?.service_name || '');
    // TLS
    setEditTlsEnabled(!!node.tls?.enabled); setEditAllowInsecure(!!node.tls?.insecure);
    setEditServerName(node.tls?.server_name || '');
    setEditAlpn(node.tls?.alpn?.join(', ') || '');
    setEditRealityEnabled(!!node.tls?.reality?.enabled);
    setEditFingerprint(node.tls?.utls?.fingerprint || 'chrome');
    setEditPublicKey(node.tls?.reality?.public_key || '');
    setEditShortId(node.tls?.reality?.short_id || '');
    setIsEditingConfig(true);
  };

  const handleSaveEditor = async () => {
    if (!selectedProfileId) return;
    setEditSaveError(null); setIsSavingConfig(true);
    if (!editTag.trim()) { setEditSaveError('Tag name is required.'); setIsSavingConfig(false); return; }

    try {
      const outbound: any = { type: editProtocol, tag: editTag.trim() };

      if (['vless','vmess','trojan','shadowsocks','hysteria2','tuic','socks','http'].includes(editProtocol)) {
        outbound.server = editAddress.trim();
        outbound.server_port = Number(editPort) || 443;
      }

      // Protocol-specific fields
      if (editProtocol === 'vless' || editProtocol === 'vmess') {
        if (!editUuid.trim()) { setEditSaveError('UUID required for VLESS/VMess.'); setIsSavingConfig(false); return; }
        outbound.uuid = editUuid.trim();
        if (editFlow && editProtocol === 'vless') outbound.flow = editFlow;
      } else if (editProtocol === 'trojan') {
        outbound.password = editPassword.trim();
      } else if (editProtocol === 'shadowsocks') {
        outbound.method = editMethod || 'aes-256-gcm';
        outbound.password = editPassword.trim();
      } else if (editProtocol === 'hysteria2') {
        outbound.password = editHy2Auth.trim();
        if (editHy2UpBw) outbound.up_mbps = parseFloat(editHy2UpBw) || undefined;
        if (editHy2DownBw) outbound.down_mbps = parseFloat(editHy2DownBw) || undefined;
        if (editHy2ObfsType) outbound.obfs = { type: editHy2ObfsType, password: editHy2ObfsPassword };
      } else if (editProtocol === 'tuic') {
        outbound.uuid = editTuicUuid.trim();
        outbound.password = editTuicPassword.trim();
        outbound.congestion_control = editTuicCongestion;
        outbound.udp_relay_mode = editTuicUdpMode;
      } else if (editProtocol === 'wireguard') {
        outbound.secret_key = editWgSecretKey.trim();
        const [epHost, epPort] = editWgEndpoint.split(':');
        outbound.peers = [{
          public_key: editWgPeerPublicKey.trim(),
          pre_shared_key: editWgPreSharedKey.trim() || undefined,
          server: epHost, server_port: parseInt(epPort) || 51820,
          allowed_ips: editWgAllowedIps.split(',').map((s) => s.trim()).filter(Boolean),
        }];
        if (editWgReserved.trim()) {
          outbound.reserved = editWgReserved.split(',').map((s) => parseInt(s.trim())).filter((n) => !isNaN(n));
        }
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

      // Transport (skip for WireGuard, SSH, SOCKS, HTTP)
      if (!['wireguard','ssh','socks','http','hysteria2','tuic'].includes(editProtocol)) {
        if (editTlsEnabled) {
          const tls: any = { enabled: true };
          if (editServerName.trim()) tls.server_name = editServerName.trim();
          if (editAllowInsecure) tls.insecure = true;
          if (editAlpn.trim()) tls.alpn = editAlpn.split(',').map((s) => s.trim()).filter(Boolean);
          if (editFingerprint) tls.utls = { enabled: true, fingerprint: editFingerprint };
          if (editRealityEnabled) {
            if (!editPublicKey.trim()) { setEditSaveError('Reality Public Key is required.'); setIsSavingConfig(false); return; }
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

      pushSystemLog(`Saving node config "${outbound.tag}"...`);
      await invoke('update_profile_config', { profileId: selectedProfileId, newOutbound: outbound });
      const outbounds = await invoke<any[]>('get_profile_outbounds', { profileId: selectedProfileId });
      setProfileOutbounds(outbounds || []);
      const active = await invoke<any>('get_profile_outbound', { profileId: selectedProfileId });
      if (active) setSelectedOutboundTag(active.tag);
      pushSystemLog(`Node "${outbound.tag}" updated and validated.`);
      setIsEditingConfig(false);
    } catch (err) {
      setEditSaveError(String(err));
      pushSystemLog(`Failed to update node: ${err}`);
    } finally { setIsSavingConfig(false); }
  };

  // ─────────────────────────────────────────────────────────────────────────────
  // ── RENDER HELPERS ────────────────────────────────────────────────────────────
  // ─────────────────────────────────────────────────────────────────────────────
  const SectionLabel = ({ children }: { children: React.ReactNode }) => (
    <div className="section-label">{children}</div>
  );

  const SubCard = ({ title, children }: { title: string; children: React.ReactNode }) => (
    <div className="editor-subcard">
      <span className="editor-subcard-title">{title}</span>
      {children}
    </div>
  );

  const BadgeOutbound = ({ action }: { action: OutboundAction }) => (
    <span className="outbound-badge" style={{ background: `${OUTBOUND_COLORS[action]}18`, color: OUTBOUND_COLORS[action], border: `1px solid ${OUTBOUND_COLORS[action]}40` }}>
      {action}
    </span>
  );

  const Toggle = ({ checked, onChange, disabled }: { checked: boolean; onChange: (v: boolean) => void; disabled?: boolean }) => (
    <label className="switch-toggle" style={disabled ? { opacity: 0.4, pointerEvents: 'none' } : undefined}>
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} disabled={disabled} />
      <span className="switch-slider"></span>
    </label>
  );

  const Inp = ({ value, onChange, placeholder, type = 'text', mono = false }: {
    value: string; onChange: (v: string) => void; placeholder?: string; type?: string; mono?: boolean;
  }) => (
    <input type={type} className="text-input" style={mono ? { fontFamily: 'var(--font-mono)', fontSize: '12px' } : undefined}
      value={value} onChange={(e) => onChange(e.target.value)} placeholder={placeholder} />
  );

  const Sel = ({ value, onChange, options }: {
    value: string; onChange: (v: string) => void;
    options: { value: string; label: string }[];
  }) => (
    <select className="select-input" value={value} onChange={(e) => onChange(e.target.value)}>
      {options.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
    </select>
  );

  // ─────────────────────────────────────────────────────────────────────────────
  // ── JSX ───────────────────────────────────────────────────────────────────────
  // ─────────────────────────────────────────────────────────────────────────────
  return (
    <div className="app-container">

      {/* ── SIDEBAR ──────────────────────────────────────────────────────────── */}
      <aside className="sidebar">
        <div className="brand-section">
          <div className="brand-logo-container">
            <Zap size={22} color="#0B0C10" strokeWidth={3} />
            <div className="brand-logo-glow" />
          </div>
          <span className="brand-title">TunX</span>
        </div>

        <nav className="nav-list">
          {([
            ['dashboard', Activity, 'Dashboard'],
            ['profiles', Layers, 'Profiles'],
            ['routing', Route, 'Routing'],
            ['dns', Dna, 'DNS'],
            ['logs', Terminal, 'Live Logs'],
            ['settings', SettingsIcon, 'Settings'],
          ] as [TabId, React.ComponentType<any>, string][]).map(([tab, Icon, label]) => (
            <div key={tab} className={`nav-item ${activeTab === tab ? 'active' : ''}`} onClick={() => setActiveTab(tab)}>
              <Icon size={18} />
              <span>{label}</span>
              {tab === 'logs' && logs.filter((l) => l.type === 'error').length > 0 && (
                <span className="nav-badge error">{logs.filter((l) => l.type === 'error').length}</span>
              )}
            </div>
          ))}
        </nav>

        <div className="sidebar-footer">
          <div className="status-badge-container">
            <div className={`status-dot ${isConnected ? 'connected' : 'idle'}`} />
            <span className="status-text" style={{ color: isConnected ? 'var(--status-connected)' : 'var(--status-idle)' }}>
              {isConnected ? 'connected' : 'idle'}
            </span>
          </div>
          {settings.apiEnabled && isConnected && (
            <div className="sidebar-api-badge">
              <Code2 size={11} /> API :{settings.apiPort}
            </div>
          )}
        </div>
      </aside>

      {/* ── MAIN CONTENT ─────────────────────────────────────────────────────── */}
      <main className="main-content">

        {/* ═══════════════════════════════════════════════════════════════════════
            VIEW 1: DASHBOARD
        ═══════════════════════════════════════════════════════════════════════ */}
        {activeTab === 'dashboard' && (
          <div className="view-container">
            <header className="view-header">
              <div>
                <h1 className="view-title">Dashboard</h1>
                <p className="view-subtitle">Real-time status monitor and proxy controls</p>
              </div>
              <div className="flex-row-between gap-12">
                {isElevated ? (
                  <div className="elev-badge">
                    <Shield size={14} /><span>Elevated (TUN Active)</span>
                  </div>
                ) : (
                  <button className="btn btn-secondary" style={{ padding: '6px 12px', borderRadius: '20px', fontSize: '12px' }} onClick={handleRequestElevation}>
                    <ShieldAlert size={14} className="text-danger" /><span>Run as Admin</span>
                  </button>
                )}
                {settings.sniffEnabled && (
                  <div className="elev-badge" style={{ background: 'rgba(168,85,247,0.08)', border: '1px solid rgba(168,85,247,0.2)', color: 'var(--accent-purple)' }}>
                    <Eye size={14} /><span>Sniffing Active</span>
                  </div>
                )}
              </div>
            </header>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
              <div className="grid-3">
                {/* Power panel */}
                <div className="glass-panel connect-panel">
                  <div className="power-button-container">
                    <div className={`power-button-outer ${isConnected ? 'connected' : ''}`} />
                    <button className={`power-button ${isConnected ? 'connected' : ''}`} onClick={handleToggleConnect}>
                      <Power />
                    </button>
                  </div>
                  <h3 className="connect-status-label">{isConnected ? 'Connected' : 'Disconnected'}</h3>
                  <p className="connect-status-sub">
                    {isConnected
                      ? `Profile: ${profiles.find((p) => p.id === activeProfileId)?.name || 'Default'}`
                      : 'Toggle power to start proxy'}
                  </p>
                  {isConnected && (
                    <div className="connect-mode-badge">
                      <Network size={12} />
                      <span>{settings.proxyMode === 'tun' ? 'TUN Mode' : 'System Proxy'}</span>
                    </div>
                  )}
                </div>

                {/* Traffic chart */}
                <div className="glass-panel chart-panel" style={{ gridColumn: 'span 2' }}>
                  <div className="chart-header">
                    <h3 className="chart-title">Bandwidth</h3>
                    <div className="chart-legends">
                      <div className="legend-item"><div className="legend-dot download" /><span>↓ {formatSpeed(downloadSpeed)}</span></div>
                      <div className="legend-item"><div className="legend-dot upload" /><span>↑ {formatSpeed(uploadSpeed)}</span></div>
                    </div>
                  </div>
                  <div className="chart-body"><TrafficChart history={speedHistory} /></div>
                </div>
              </div>

              {/* Metrics */}
              <div className="grid-4">
                {[
                  { icon: DownloadCloud, label: 'Total Down', value: formatBytes(downloadBytes), color: 'var(--accent-cyan)' },
                  { icon: UploadCloud, label: 'Total Up', value: formatBytes(uploadBytes), color: 'var(--accent-purple)' },
                  { icon: Clock, label: 'Uptime', value: formatUptime(uptime), color: 'var(--status-idle)' },
                  { icon: Server, label: 'Connections', value: `${activeConnections} active`, color: 'var(--status-connected)' },
                ].map(({ icon: Icon, label, value, color }) => (
                  <div key={label} className="glass-panel metric-card">
                    <div className="metric-icon-box" style={{ color }}><Icon size={20} /></div>
                    <div className="metric-info">
                      <span className="metric-label">{label}</span>
                      <span className="metric-value">{value}</span>
                    </div>
                  </div>
                ))}
              </div>

              {/* Ports info bar */}
              <div className="glass-panel" style={{ padding: '16px 24px' }}>
                <div className="flex-row-between">
                  <h3 style={{ fontSize: '14px', fontWeight: 600 }}>Active Inbound Ports</h3>
                  <div style={{ display: 'flex', gap: '24px', fontSize: '13px' }}>
                    {[
                      { label: 'HTTP', port: httpPort },
                      { label: 'SOCKS5', port: socksPort },
                      { label: 'Mixed', port: mixedPort },
                      ...(settings.apiEnabled ? [{ label: 'API', port: settings.apiPort }] : []),
                    ].map(({ label, port }) => (
                      <span key={label}>
                        <span style={{ color: 'var(--text-secondary)' }}>{label}: </span>
                        <strong style={{ color: 'var(--accent-cyan)' }}>{port}</strong>
                      </span>
                    ))}
                    {settings.wifiSharing && (
                      <span style={{ color: 'var(--status-connected)', display: 'flex', alignItems: 'center', gap: '4px', fontSize: '12px' }}>
                        <Wifi size={12} /> LAN Sharing On
                      </span>
                    )}
                  </div>
                </div>
              </div>

              {/* Outbound node selector */}
              <div className="glass-panel">
                <div className="flex-row-between" style={{ marginBottom: '16px' }}>
                  <h3 style={{ fontSize: '16px', fontWeight: 600 }}>Active Profile Outbounds</h3>
                </div>
                {selectedProfileId ? (
                  <>
                    <p style={{ fontSize: '13px', color: 'var(--text-secondary)', marginBottom: '12px' }}>
                      Select proxy outbound node for traffic routing:
                    </p>
                    {profileOutbounds.length > 0 ? (
                      <div className="node-grid">
                        {profileOutbounds.map((node, i) => (
                          <div key={i} className={`node-card ${selectedOutboundTag === node.tag ? 'active' : ''}`} onClick={() => handleSelectOutbound(node)}>
                            <span className="node-name" title={node.tag}>{node.tag}</span>
                            <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                              <span className="node-type-badge">{node.type}</span>
                              <button className="btn-icon-only" style={{ width: '22px', height: '22px', border: 'none', background: 'transparent', color: 'var(--text-secondary)' }}
                                onClick={(e) => { e.stopPropagation(); handleOpenEditor(node); }} title="Edit node">
                                <Edit3 size={11} />
                              </button>
                            </div>
                          </div>
                        ))}
                      </div>
                    ) : (
                      <p style={{ fontSize: '13px', color: 'var(--text-muted)', marginTop: '8px' }}>
                        No customizable outbounds found. All traffic handled by profile rules.
                      </p>
                    )}
                  </>
                ) : (
                  <p style={{ fontSize: '13px', color: 'var(--text-muted)' }}>No profile selected. Go to Profiles to add one.</p>
                )}
              </div>
            </div>
          </div>
        )}

        {/* ═══════════════════════════════════════════════════════════════════════
            VIEW 2: PROFILES
        ═══════════════════════════════════════════════════════════════════════ */}
        {activeTab === 'profiles' && (
          <div className="view-container">
            <header className="view-header">
              <div><h1 className="view-title">Profiles</h1><p className="view-subtitle">Import and manage proxy subscription profiles</p></div>
            </header>
            <div className="grid-2">
              <div className="glass-panel" style={{ height: 'calc(100vh - 180px)', display: 'flex', flexDirection: 'column' }}>
                <h3 style={{ fontSize: '16px', fontWeight: 600, marginBottom: '16px' }}>Your Configs</h3>
                <div style={{ flexGrow: 1, overflowY: 'auto', paddingRight: '4px' }}>
                  {profiles.length > 0 ? profiles.map((p) => (
                    <div key={p.id} className={`profile-item ${selectedProfileId === p.id ? 'active' : ''}`} onClick={() => setSelectedProfileId(p.id)}>
                      <div className="profile-item-left">
                        <div className="profile-icon"><Globe size={18} /></div>
                        <div className="profile-details">
                          <span className="profile-name">{p.name}</span>
                          <div className="profile-meta">
                            <span style={{ textTransform: 'capitalize' }}>{p.type}</span>
                            <span>•</span><span>{p.nodeCount} nodes</span>
                          </div>
                        </div>
                      </div>
                      <div className="profile-actions">
                        {activeProfileId === p.id && isConnected && (
                          <span className="active-badge">ACTIVE</span>
                        )}
                        <button className="btn-icon-only danger" onClick={(e) => handleDeleteProfile(p.id, e)} title="Delete">
                          <Trash2 size={15} />
                        </button>
                      </div>
                    </div>
                  )) : (
                    <div style={{ textAlign: 'center', color: 'var(--text-muted)', marginTop: '60px' }}>
                      <Globe size={40} style={{ strokeWidth: 1, marginBottom: '12px' }} />
                      <p style={{ fontSize: '14px' }}>No profiles found</p>
                      <p style={{ fontSize: '12px', marginTop: '4px' }}>Import a config using the panel on the right.</p>
                    </div>
                  )}
                </div>
              </div>

              <div className="glass-panel" style={{ height: 'calc(100vh - 180px)', display: 'flex', flexDirection: 'column' }}>
                <h3 style={{ fontSize: '16px', fontWeight: 600, marginBottom: '16px' }}>Import Profile</h3>
                <div className="tab-row">
                  {[['url','Subscription URL'],['file','Local File'],['clipboard','Clipboard']] .map(([id, label]) => (
                    <button key={id} className={`tab-btn ${importTab === id ? 'active' : ''}`} onClick={() => setImportTab(id as any)}>{label}</button>
                  ))}
                </div>
                <form onSubmit={handleImportProfile} style={{ display: 'flex', flexDirection: 'column', gap: '16px', flexGrow: 1 }}>
                  <div className="form-group">
                    <label className="form-label">Profile Name</label>
                    <Inp value={importName} onChange={setImportName} placeholder="e.g. Premium Proxy" />
                  </div>
                  {importTab === 'url' && (
                    <div className="form-group">
                      <label className="form-label">Subscription Link</label>
                      <Inp value={importUrl} onChange={setImportUrl} placeholder="https://provider.com/clash.yaml" type="url" />
                    </div>
                  )}
                  {importTab === 'file' && (
                    <div className="form-group">
                      <label className="form-label">Configuration File</label>
                      <div style={{ display: 'flex', gap: '10px' }}>
                        <Inp value={importFilePath} onChange={() => {}} placeholder="Select JSON, YAML, or Base64 file..." />
                        <button type="button" className="btn btn-secondary" onClick={handlePickFile}><FolderOpen size={16} /> Browse</button>
                      </div>
                    </div>
                  )}
                  {importTab === 'clipboard' && (
                    <div className="form-group" style={{ flexGrow: 1, display: 'flex', flexDirection: 'column' }}>
                      <div className="flex-row-between">
                        <label className="form-label">Raw Config (Clash/yaml or base64)</label>
                        <button type="button" className="btn btn-secondary" style={{ padding: '4px 8px', fontSize: '11px' }} onClick={handlePasteClipboard}>
                          <Clipboard size={12} /> Paste
                        </button>
                      </div>
                      <textarea className="text-input" style={{ height: '140px', fontFamily: 'var(--font-mono)', fontSize: '12px', resize: 'none', flexGrow: 1, marginTop: '8px' }}
                        value={importContent} onChange={(e) => setImportContent(e.target.value)}
                        placeholder="Paste subscription bytes, Clash YAML, or sing-box JSON here..." />
                    </div>
                  )}
                  {importError && (
                    <div className="alert-box error"><ShieldAlert size={14} /><span>{importError}</span></div>
                  )}
                  {importSuccess && (
                    <div className="alert-box success"><Check size={14} /><span>Profile imported and validated!</span></div>
                  )}
                  <button type="submit" className="btn btn-primary" disabled={isImporting} style={{ marginTop: 'auto', width: '100%', height: '44px' }}>
                    {isImporting ? <><RefreshCw size={16} className="spin" /> Validating...</> : <><Plus size={18} /> Import & Validate</>}
                  </button>
                </form>
              </div>
            </div>
          </div>
        )}

        {/* ═══════════════════════════════════════════════════════════════════════
            VIEW 3: ROUTING
        ═══════════════════════════════════════════════════════════════════════ */}
        {activeTab === 'routing' && (
          <div className="view-container">
            <header className="view-header">
              <div>
                <h1 className="view-title">Routing</h1>
                <p className="view-subtitle">Advanced traffic routing engine — GeoIP, GeoSite, rule sets &amp; split tunneling</p>
              </div>
            </header>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
              {/* ── Global Routing Settings ── */}
              <div className="grid-3">
                <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '4px' }}>
                    <GitFork size={18} style={{ color: 'var(--accent-cyan)' }} />
                    <h3 style={{ fontSize: '15px', fontWeight: 600 }}>Global Routing</h3>
                  </div>
                  <div className="form-group" style={{ marginBottom: 0 }}>
                    <label className="form-label">Final Outbound (Fallback)</label>
                    <Sel value={settings.finalOutbound} onChange={(v) => handleSaveSettings({ finalOutbound: v as OutboundAction })}
                      options={[{ value: 'proxy', label: '🌐 Proxy (default)' }, { value: 'direct', label: '⚡ Direct' }, { value: 'block', label: '🚫 Block' }]} />
                  </div>
                  <div className="switch-container" style={{ padding: '10px 14px' }}>
                    <div className="switch-details">
                      <span className="switch-title" style={{ fontSize: '13px' }}>Bypass LAN / Private IPs</span>
                      <span className="switch-desc">Route RFC-1918 addresses directly</span>
                    </div>
                    <Toggle checked={settings.bypassLan} onChange={(v) => handleSaveSettings({ bypassLan: v })} />
                  </div>
                  <div className="switch-container" style={{ padding: '10px 14px' }}>
                    <div className="switch-details">
                      <span className="switch-title" style={{ fontSize: '13px' }}>Bypass China (GeoIP/GeoSite)</span>
                      <span className="switch-desc">Route CN addresses directly</span>
                    </div>
                    <Toggle checked={settings.bypassChina} onChange={(v) => handleSaveSettings({ bypassChina: v })} />
                  </div>
                </div>

                <div className="glass-panel" style={{ gridColumn: 'span 2' }}>
                  <div className="flex-row-between" style={{ marginBottom: '16px' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                      <Database size={18} style={{ color: 'var(--accent-cyan)' }} />
                      <h3 style={{ fontSize: '15px', fontWeight: 600 }}>Rule Sets</h3>
                    </div>
                    <button className="btn btn-secondary" style={{ padding: '6px 12px', fontSize: '12px' }} onClick={() => setShowAddRuleSet(true)}>
                      <Plus size={14} /> Add Rule Set
                    </button>
                  </div>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', maxHeight: '220px', overflowY: 'auto' }}>
                    {ruleSets.map((rs) => (
                      <div key={rs.id} className="rule-row">
                        <div style={{ display: 'flex', alignItems: 'center', gap: '10px', minWidth: 0 }}>
                          <Database size={13} style={{ color: 'var(--text-muted)', flexShrink: 0 }} />
                          <span style={{ fontWeight: 600, fontSize: '13px', color: 'var(--accent-cyan)', fontFamily: 'var(--font-mono)' }}>{rs.tag}</span>
                          <span className="type-chip">{rs.type}</span>
                          <span className="type-chip" style={{ background: 'rgba(168,85,247,0.1)', color: '#d8b4fe', border: '1px solid rgba(168,85,247,0.2)' }}>{rs.format}</span>
                          <span className="type-chip">↻ {rs.updateInterval}</span>
                        </div>
                        <div style={{ display: 'flex', gap: '6px', alignItems: 'center', flexShrink: 0 }}>
                          {rs.lastUpdated && <span style={{ fontSize: '10px', color: 'var(--text-muted)' }}>{new Date(rs.lastUpdated).toLocaleDateString()}</span>}
                          {rs.type === 'remote' && (
                            <button className="btn-icon-only" style={{ width: '26px', height: '26px' }} onClick={() => handleUpdateRuleSet(rs)} title="Update now" disabled={updatingRuleSet === rs.id}>
                              <Download size={12} className={updatingRuleSet === rs.id ? 'spin' : ''} />
                            </button>
                          )}
                          <button className="btn-icon-only danger" style={{ width: '26px', height: '26px' }} onClick={() => handleDeleteRuleSet(rs.id)} title="Delete">
                            <Trash2 size={12} />
                          </button>
                        </div>
                      </div>
                    ))}
                    {ruleSets.length === 0 && (
                      <div style={{ textAlign: 'center', color: 'var(--text-muted)', padding: '24px', fontSize: '13px' }}>
                        No rule sets. Add a remote GeoIP/GeoSite rule set above.
                      </div>
                    )}
                  </div>
                </div>
              </div>

              {/* ── Add Rule Set Form ── */}
              {showAddRuleSet && (
                <div className="glass-panel" style={{ border: '1px solid rgba(102,252,241,0.2)' }}>
                  <div className="flex-row-between" style={{ marginBottom: '16px' }}>
                    <h3 style={{ fontSize: '15px', fontWeight: 600 }}>Add Rule Set</h3>
                    <button className="btn-icon-only" style={{ border: 'none', background: 'transparent' }} onClick={() => setShowAddRuleSet(false)}><X size={16} /></button>
                  </div>
                  <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr 120px 120px', gap: '12px', alignItems: 'end' }}>
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">Tag</label>
                      <Inp value={ruleSetForm.tag} onChange={(v) => setRuleSetForm((p) => ({ ...p, tag: v }))} placeholder="geoip-cn" mono />
                    </div>
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">Type</label>
                      <Sel value={ruleSetForm.type} onChange={(v) => setRuleSetForm((p) => ({ ...p, type: v as any }))} options={[{ value: 'remote', label: 'Remote' }, { value: 'local', label: 'Local' }]} />
                    </div>
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">Format</label>
                      <Sel value={ruleSetForm.format} onChange={(v) => setRuleSetForm((p) => ({ ...p, format: v as any }))} options={[{ value: 'binary', label: 'Binary (.srs)' }, { value: 'source', label: 'Source (.json)' }]} />
                    </div>
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">Update Every</label>
                      <Sel value={ruleSetForm.updateInterval} onChange={(v) => setRuleSetForm((p) => ({ ...p, updateInterval: v }))} options={[{ value: '1h', label: '1 Hour' }, { value: '12h', label: '12 Hours' }, { value: '1d', label: '1 Day' }, { value: '7d', label: '7 Days' }]} />
                    </div>
                    <button className="btn btn-primary" style={{ height: '42px' }} onClick={handleSaveRuleSet}>Add</button>
                  </div>
                  {ruleSetForm.type === 'remote' && (
                    <div className="form-group" style={{ marginTop: '12px', marginBottom: 0 }}>
                      <label className="form-label">Remote URL</label>
                      <Inp value={ruleSetForm.url || ''} onChange={(v) => setRuleSetForm((p) => ({ ...p, url: v }))} placeholder="https://cdn.jsdelivr.net/.../geoip/cn.srs" />
                    </div>
                  )}
                </div>
              )}

              {/* ── Routing Rules ── */}
              <div className="glass-panel">
                <div className="flex-row-between" style={{ marginBottom: '16px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                    <Filter size={18} style={{ color: 'var(--accent-cyan)' }} />
                    <h3 style={{ fontSize: '15px', fontWeight: 600 }}>Routing Rules</h3>
                    <span className="type-chip">{routingRules.length} rules</span>
                  </div>
                  <button className="btn btn-secondary" style={{ padding: '6px 12px', fontSize: '12px' }} onClick={() => { setEditingRule(null); setRuleForm({ type: 'geoip', value: '', outbound: 'direct', invert: false, notes: '' }); setShowAddRule(true); }}>
                    <Plus size={14} /> Add Rule
                  </button>
                </div>

                {/* Inline add/edit form */}
                {showAddRule && (
                  <div className="add-rule-form">
                    <div style={{ display: 'grid', gridTemplateColumns: '160px 1fr 140px 80px 1fr auto', gap: '10px', alignItems: 'end' }}>
                      <div className="form-group" style={{ marginBottom: 0 }}>
                        <label className="form-label">Rule Type</label>
                        <Sel value={ruleForm.type} onChange={(v) => setRuleForm((p) => ({ ...p, type: v as RoutingRuleType }))} options={[
                          { value: 'geoip', label: 'GeoIP' }, { value: 'geosite', label: 'GeoSite' },
                          { value: 'domain', label: 'Domain' }, { value: 'domain_suffix', label: 'Domain Suffix' },
                          { value: 'domain_keyword', label: 'Domain Keyword' }, { value: 'domain_regex', label: 'Domain Regex' },
                          { value: 'ip_cidr', label: 'IP CIDR' }, { value: 'port', label: 'Port' },
                          { value: 'port_range', label: 'Port Range' }, { value: 'protocol', label: 'Protocol' },
                          { value: 'process_name', label: 'Process Name' }, { value: 'rule_set', label: 'Rule Set' },
                          { value: 'network', label: 'Network (tcp/udp)' }, { value: 'package_name', label: 'Package Name (Android)' },
                        ]} />
                      </div>
                      <div className="form-group" style={{ marginBottom: 0 }}>
                        <label className="form-label">Value</label>
                        <Inp value={ruleForm.value} onChange={(v) => setRuleForm((p) => ({ ...p, value: v }))} placeholder={
                          ruleForm.type === 'geoip' ? 'cn, us, private...' :
                          ruleForm.type === 'ip_cidr' ? '10.0.0.0/8' :
                          ruleForm.type === 'port' ? '443' :
                          ruleForm.type === 'rule_set' ? 'geoip-cn' :
                          'e.g. google.com'
                        } mono />
                      </div>
                      <div className="form-group" style={{ marginBottom: 0 }}>
                        <label className="form-label">Outbound</label>
                        <Sel value={ruleForm.outbound} onChange={(v) => setRuleForm((p) => ({ ...p, outbound: v as OutboundAction }))} options={[
                          { value: 'proxy', label: '🌐 Proxy' }, { value: 'direct', label: '⚡ Direct' },
                          { value: 'block', label: '🚫 Block' }, { value: 'dns', label: '🔍 DNS' },
                        ]} />
                      </div>
                      <div className="form-group" style={{ marginBottom: 0, alignItems: 'center' }}>
                        <label className="form-label">Invert</label>
                        <div style={{ display: 'flex', justifyContent: 'center', paddingTop: '6px' }}>
                          <Toggle checked={ruleForm.invert} onChange={(v) => setRuleForm((p) => ({ ...p, invert: v }))} />
                        </div>
                      </div>
                      <div className="form-group" style={{ marginBottom: 0 }}>
                        <label className="form-label">Notes (optional)</label>
                        <Inp value={ruleForm.notes || ''} onChange={(v) => setRuleForm((p) => ({ ...p, notes: v }))} placeholder="e.g. China bypass" />
                      </div>
                      <div style={{ display: 'flex', gap: '6px', paddingBottom: '1px' }}>
                        <button className="btn btn-primary" style={{ padding: '10px 16px', height: '42px' }} onClick={handleSaveRoutingRule}>
                          {editingRule ? 'Update' : 'Add'}
                        </button>
                        <button className="btn btn-secondary" style={{ padding: '10px 12px', height: '42px' }} onClick={() => { setShowAddRule(false); setEditingRule(null); }}>
                          <X size={16} />
                        </button>
                      </div>
                    </div>
                  </div>
                )}

                {/* Rules table */}
                <div className="rules-table">
                  <div className="rules-table-header">
                    <span style={{ width: '30px' }}>#</span>
                    <span style={{ flex: '0 0 140px' }}>Type</span>
                    <span style={{ flex: 1 }}>Value</span>
                    <span style={{ flex: '0 0 100px' }}>Outbound</span>
                    <span style={{ flex: 1 }}>Notes</span>
                    <span style={{ flex: '0 0 70px', textAlign: 'right' }}>Actions</span>
                  </div>
                  {routingRules.map((rule, i) => (
                    <div key={rule.id} className="rules-table-row">
                      <span style={{ width: '30px', color: 'var(--text-muted)', fontSize: '12px' }}>{i + 1}</span>
                      <span style={{ flex: '0 0 140px' }}>
                        <span className="type-chip" style={{ fontFamily: 'var(--font-mono)' }}>{rule.type}{rule.invert ? ' !' : ''}</span>
                      </span>
                      <span style={{ flex: 1, fontFamily: 'var(--font-mono)', fontSize: '12px', color: 'var(--text-primary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{rule.value}</span>
                      <span style={{ flex: '0 0 100px' }}><BadgeOutbound action={rule.outbound} /></span>
                      <span style={{ flex: 1, fontSize: '11px', color: 'var(--text-muted)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{rule.notes || '—'}</span>
                      <span style={{ flex: '0 0 70px', display: 'flex', gap: '4px', justifyContent: 'flex-end' }}>
                        <button className="btn-icon-only" style={{ width: '24px', height: '24px' }} onClick={() => { setEditingRule(rule); setRuleForm({ type: rule.type, value: rule.value, outbound: rule.outbound, invert: rule.invert, notes: rule.notes }); setShowAddRule(true); }}>
                          <Edit3 size={11} />
                        </button>
                        <button className="btn-icon-only danger" style={{ width: '24px', height: '24px' }} onClick={() => handleDeleteRoutingRule(rule.id)}>
                          <Trash2 size={11} />
                        </button>
                      </span>
                    </div>
                  ))}
                  {routingRules.length === 0 && (
                    <div style={{ textAlign: 'center', color: 'var(--text-muted)', padding: '40px 0', fontSize: '13px' }}>
                      No routing rules. Traffic falls through to final outbound ({settings.finalOutbound}).
                    </div>
                  )}
                </div>

                <div className="info-box" style={{ marginTop: '16px' }}>
                  <Info size={13} />
                  <span>Rules are evaluated top-to-bottom. The first match wins. Rules generate the sing-box <code>route.rules</code> array in config order.</span>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* ═══════════════════════════════════════════════════════════════════════
            VIEW 4: DNS
        ═══════════════════════════════════════════════════════════════════════ */}
        {activeTab === 'dns' && (
          <div className="view-container">
            <header className="view-header">
              <div>
                <h1 className="view-title">DNS</h1>
                <p className="view-subtitle">DNS engine — DoH, DoT, DoQ, FakeIP, DNS rules &amp; leak protection</p>
              </div>
            </header>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
              <div className="grid-2">
                {/* DNS Mode & Options */}
                <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '4px' }}>
                    <Dna size={18} style={{ color: 'var(--accent-cyan)' }} />
                    <h3 style={{ fontSize: '15px', fontWeight: 600 }}>DNS Mode &amp; Options</h3>
                  </div>

                  <div className="form-group" style={{ marginBottom: 0 }}>
                    <label className="form-label">DNS Resolution Mode</label>
                    <Sel value={settings.dnsMode} onChange={(v) => handleSaveSettings({ dnsMode: v as DnsMode })} options={[
                      { value: 'fakeip', label: '👻 FakeIP (Recommended for TUN)' },
                      { value: 'normal', label: '🔍 Normal (Standard DNS)' },
                    ]} />
                    <span style={{ fontSize: '12px', color: 'var(--text-muted)', lineHeight: 1.5, display: 'block', marginTop: '4px' }}>
                      {settings.dnsMode === 'fakeip' ? 'FakeIP assigns fake addresses for domains, enabling fast connection hijacking through TUN.' : 'Normal mode resolves DNS normally through configured servers.'}
                    </span>
                  </div>

                  <div className="form-group" style={{ marginBottom: 0 }}>
                    <label className="form-label">IP Strategy</label>
                    <Sel value={settings.dnsStrategy} onChange={(v) => handleSaveSettings({ dnsStrategy: v as DnsStrategy })} options={[
                      { value: 'prefer_ipv4', label: 'Prefer IPv4' }, { value: 'prefer_ipv6', label: 'Prefer IPv6' },
                      { value: 'ipv4_only', label: 'IPv4 Only' }, { value: 'ipv6_only', label: 'IPv6 Only' },
                    ]} />
                  </div>

                  {[
                    { key: 'dnsLeakProtection', title: 'DNS Leak Protection', desc: 'Forces DNS queries through the tunnel to prevent ISP DNS exposure', icon: ShieldCheck },
                    { key: 'dnsCaching', title: 'DNS Response Caching', desc: 'Cache DNS responses to reduce latency and lookup overhead', icon: Database },
                  ].map(({ key, title, desc, icon: Icon }) => (
                    <div key={key} className="switch-container" style={{ padding: '10px 14px' }}>
                      <div className="switch-details">
                        <span className="switch-title" style={{ fontSize: '13px', display: 'flex', alignItems: 'center', gap: '6px' }}>
                          <Icon size={13} style={{ color: 'var(--accent-cyan)' }} /> {title}
                        </span>
                        <span className="switch-desc">{desc}</span>
                      </div>
                      <Toggle checked={(settings as any)[key]} onChange={(v) => handleSaveSettings({ [key]: v } as any)} />
                    </div>
                  ))}
                </div>

                {/* DNS Servers */}
                <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '4px' }}>
                    <Server size={18} style={{ color: 'var(--accent-cyan)' }} />
                    <h3 style={{ fontSize: '15px', fontWeight: 600 }}>DNS Servers</h3>
                  </div>

                  {[
                    { key: 'primaryDns', label: 'Primary DNS (Encrypted)', placeholder: 'https://1.1.1.1/dns-query', badge: 'DoH', badgeColor: 'var(--accent-cyan)' },
                    { key: 'fallbackDns', label: 'Fallback DNS', placeholder: 'https://8.8.8.8/dns-query', badge: 'DoH', badgeColor: 'var(--accent-purple)' },
                    { key: 'directDns', label: 'Direct DNS (ISP / Router)', placeholder: '192.168.1.1 or 223.5.5.5', badge: 'UDP', badgeColor: 'var(--status-connected)' },
                  ].map(({ key, label, placeholder, badge, badgeColor }) => (
                    <div key={key} className="form-group" style={{ marginBottom: 0 }}>
                      <div className="flex-row-between">
                        <label className="form-label">{label}</label>
                        <span style={{ fontSize: '10px', fontWeight: 700, padding: '2px 6px', borderRadius: '4px', background: `${badgeColor}18`, color: badgeColor, border: `1px solid ${badgeColor}40` }}>{badge}</span>
                      </div>
                      <Inp value={(settings as any)[key]} onChange={(v) => handleSaveSettings({ [key]: v } as any)} placeholder={placeholder} mono />
                    </div>
                  ))}

                  <div className="info-box">
                    <Info size={13} />
                    <span>Protocols: <code>https://</code> = DoH · <code>tls://</code> = DoT · <code>quic://</code> = DoQ · plain IP = UDP</span>
                  </div>
                </div>
              </div>

              {/* FakeIP Config (shown only in fakeip mode) */}
              {settings.dnsMode === 'fakeip' && (
                <div className="glass-panel">
                  <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '16px' }}>
                    <Ghost size={18} style={{ color: 'var(--accent-purple)' }} />
                    <h3 style={{ fontSize: '15px', fontWeight: 600 }}>FakeIP Configuration</h3>
                  </div>
                  <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px' }}>
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">FakeIP CIDR Range</label>
                      <Inp value={settings.fakeipRange} onChange={(v) => handleSaveSettings({ fakeipRange: v })} placeholder="198.18.0.0/15" mono />
                      <span style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '4px', display: 'block' }}>IP range allocated for fake IP addresses. Must not overlap your LAN.</span>
                    </div>
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">FakeIP Filter (bypass FakeIP)</label>
                      <Inp value={settings.fakeipFilter} onChange={(v) => handleSaveSettings({ fakeipFilter: v })} placeholder="geosite:private" mono />
                      <span style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '4px', display: 'block' }}>Domains in this set are excluded from FakeIP (resolved normally).</span>
                    </div>
                  </div>
                </div>
              )}

              {/* DNS Rules */}
              <div className="glass-panel">
                <div className="flex-row-between" style={{ marginBottom: '16px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                    <Filter size={18} style={{ color: 'var(--accent-cyan)' }} />
                    <h3 style={{ fontSize: '15px', fontWeight: 600 }}>DNS Rules</h3>
                    <span className="type-chip">{dnsRules.length} rules</span>
                  </div>
                  <button className="btn btn-secondary" style={{ padding: '6px 12px', fontSize: '12px' }} onClick={() => { setEditingDnsRule(null); setDnsRuleForm({ type: 'geosite', value: '', server: 'direct', disableCache: false, invert: false }); setShowAddDnsRule(true); }}>
                    <Plus size={14} /> Add DNS Rule
                  </button>
                </div>

                {showAddDnsRule && (
                  <div className="add-rule-form">
                    <div style={{ display: 'grid', gridTemplateColumns: '160px 1fr 160px 80px auto', gap: '10px', alignItems: 'end' }}>
                      <div className="form-group" style={{ marginBottom: 0 }}>
                        <label className="form-label">Type</label>
                        <Sel value={dnsRuleForm.type} onChange={(v) => setDnsRuleForm((p) => ({ ...p, type: v as DnsRuleType }))} options={[
                          { value: 'geosite', label: 'GeoSite' }, { value: 'domain', label: 'Domain' },
                          { value: 'domain_suffix', label: 'Domain Suffix' }, { value: 'domain_keyword', label: 'Domain Keyword' },
                          { value: 'rule_set', label: 'Rule Set' }, { value: 'ip_cidr', label: 'IP CIDR' },
                        ]} />
                      </div>
                      <div className="form-group" style={{ marginBottom: 0 }}>
                        <label className="form-label">Value</label>
                        <Inp value={dnsRuleForm.value} onChange={(v) => setDnsRuleForm((p) => ({ ...p, value: v }))} placeholder="cn, private, google.com..." mono />
                      </div>
                      <div className="form-group" style={{ marginBottom: 0 }}>
                        <label className="form-label">Use DNS Server</label>
                        <Sel value={dnsRuleForm.server} onChange={(v) => setDnsRuleForm((p) => ({ ...p, server: v }))} options={[
                          { value: 'primary', label: 'Primary (Encrypted)' },
                          { value: 'fallback', label: 'Fallback' },
                          { value: 'direct', label: 'Direct (ISP)' },
                          { value: 'block', label: 'Block (NXDOMAIN)' },
                        ]} />
                      </div>
                      <div className="form-group" style={{ marginBottom: 0, alignItems: 'center' }}>
                        <label className="form-label">No Cache</label>
                        <div style={{ paddingTop: '6px', display: 'flex', justifyContent: 'center' }}>
                          <Toggle checked={dnsRuleForm.disableCache} onChange={(v) => setDnsRuleForm((p) => ({ ...p, disableCache: v }))} />
                        </div>
                      </div>
                      <div style={{ display: 'flex', gap: '6px', paddingBottom: '1px' }}>
                        <button className="btn btn-primary" style={{ padding: '10px 16px', height: '42px' }} onClick={handleSaveDnsRule}>
                          {editingDnsRule ? 'Update' : 'Add'}
                        </button>
                        <button className="btn btn-secondary" style={{ padding: '10px 12px', height: '42px' }} onClick={() => { setShowAddDnsRule(false); setEditingDnsRule(null); }}>
                          <X size={16} />
                        </button>
                      </div>
                    </div>
                  </div>
                )}

                <div className="rules-table">
                  <div className="rules-table-header">
                    <span style={{ width: '30px' }}>#</span>
                    <span style={{ flex: '0 0 140px' }}>Type</span>
                    <span style={{ flex: 1 }}>Value</span>
                    <span style={{ flex: '0 0 150px' }}>Server</span>
                    <span style={{ flex: '0 0 80px' }}>Cache</span>
                    <span style={{ flex: '0 0 70px', textAlign: 'right' }}>Actions</span>
                  </div>
                  {dnsRules.map((rule, i) => (
                    <div key={rule.id} className="rules-table-row">
                      <span style={{ width: '30px', color: 'var(--text-muted)', fontSize: '12px' }}>{i + 1}</span>
                      <span style={{ flex: '0 0 140px' }}><span className="type-chip" style={{ fontFamily: 'var(--font-mono)' }}>{rule.type}</span></span>
                      <span style={{ flex: 1, fontFamily: 'var(--font-mono)', fontSize: '12px', color: 'var(--text-primary)' }}>{rule.value}</span>
                      <span style={{ flex: '0 0 150px' }}>
                        <span className="type-chip" style={{
                          background: rule.server === 'direct' ? 'rgba(16,185,129,0.1)' : rule.server === 'block' ? 'rgba(239,68,68,0.1)' : 'rgba(102,252,241,0.1)',
                          color: rule.server === 'direct' ? 'var(--status-connected)' : rule.server === 'block' ? 'var(--status-error)' : 'var(--accent-cyan)',
                          border: `1px solid ${rule.server === 'direct' ? 'rgba(16,185,129,0.2)' : rule.server === 'block' ? 'rgba(239,68,68,0.2)' : 'rgba(102,252,241,0.2)'}`,
                        }}>{rule.server}</span>
                      </span>
                      <span style={{ flex: '0 0 80px', fontSize: '12px', color: rule.disableCache ? 'var(--status-error)' : 'var(--status-connected)' }}>
                        {rule.disableCache ? '✗ off' : '✓ on'}
                      </span>
                      <span style={{ flex: '0 0 70px', display: 'flex', gap: '4px', justifyContent: 'flex-end' }}>
                        <button className="btn-icon-only" style={{ width: '24px', height: '24px' }} onClick={() => { setEditingDnsRule(rule); setDnsRuleForm({ type: rule.type, value: rule.value, server: rule.server, disableCache: rule.disableCache, invert: rule.invert }); setShowAddDnsRule(true); }}>
                          <Edit3 size={11} />
                        </button>
                        <button className="btn-icon-only danger" style={{ width: '24px', height: '24px' }} onClick={() => handleDeleteDnsRule(rule.id)}>
                          <Trash2 size={11} />
                        </button>
                      </span>
                    </div>
                  ))}
                  {dnsRules.length === 0 && (
                    <div style={{ textAlign: 'center', color: 'var(--text-muted)', padding: '40px 0', fontSize: '13px' }}>
                      No DNS rules. All domains resolve via the primary DNS server.
                    </div>
                  )}
                </div>
              </div>
            </div>
          </div>
        )}

        {/* ═══════════════════════════════════════════════════════════════════════
            VIEW 5: LOGS
        ═══════════════════════════════════════════════════════════════════════ */}
        {activeTab === 'logs' && (
          <div className="view-container">
            <header className="view-header">
              <div><h1 className="view-title">Live Logs</h1><p className="view-subtitle">Real-time sing-box process console output</p></div>
            </header>
            <div className="glass-panel console-panel">
              <div className="console-header">
                <div className="console-title-box">
                  <Terminal size={18} className="text-success" />
                  <span style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-secondary)' }}>sing-box output terminal</span>
                  <span className="type-chip">{logs.length} lines</span>
                </div>
                <div style={{ display: 'flex', gap: '12px' }}>
                  <button className="btn btn-secondary" style={{ padding: '6px 12px', fontSize: '12px' }} onClick={() => { navigator.clipboard.writeText(logs.map((l) => l.text).join('\n')); pushSystemLog('Copied all logs to clipboard.'); }}>
                    <Copy size={14} /> Copy Logs
                  </button>
                  <button className="btn btn-secondary" style={{ padding: '6px 12px', fontSize: '12px' }} onClick={() => setLogs([])}>
                    Clear Console
                  </button>
                  <Toggle checked={autoScrollLogs} onChange={setAutoScrollLogs} />
                  <span style={{ fontSize: '12px', color: 'var(--text-secondary)', margin: 'auto 0 auto 4px' }}>Autoscroll</span>
                </div>
              </div>
              <div className="console-terminal">
                {logs.length > 0 ? logs.map((log, i) => (
                  <div key={i} className={`console-line ${log.type}`}>{log.text}</div>
                )) : (
                  <div style={{ textAlign: 'center', color: 'var(--text-muted)', height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center', flexDirection: 'column' }}>
                    <Terminal size={32} style={{ strokeWidth: 1, marginBottom: '8px' }} />
                    <span>Waiting for live process logs...</span>
                  </div>
                )}
                <div ref={consoleEndRef} />
              </div>
            </div>
          </div>
        )}

        {/* ═══════════════════════════════════════════════════════════════════════
            VIEW 6: SETTINGS (EXPANDED)
        ═══════════════════════════════════════════════════════════════════════ */}
        {activeTab === 'settings' && (
          <div className="view-container">
            <header className="view-header">
              <div><h1 className="view-title">Settings</h1><p className="view-subtitle">Proxy mode, TUN, sniffing, multiplexing, API and performance options</p></div>
            </header>

            {/* Settings sub-tabs */}
            <div className="tab-row" style={{ marginBottom: '24px' }}>
              {[
                ['general', SettingsIcon, 'General'],
                ['tun', Network, 'TUN Mode'],
                ['sniff', Eye, 'Sniffing'],
                ['mux', Layers2, 'Multiplexing'],
                ['api', Code2, 'API'],
              ].map(([id, Icon, label]: any) => (
                <button key={id} className={`tab-btn ${settingsTab === id ? 'active' : ''}`} onClick={() => setSettingsTab(id)}>
                  <Icon size={13} style={{ display: 'inline', marginRight: '6px' }} />{label}
                </button>
              ))}
            </div>

            {/* ── General ── */}
            {settingsTab === 'general' && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
                <div className="grid-2">
                  <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                    <SectionLabel>Proxy Modes &amp; Behaviors</SectionLabel>
                    {[
                      { key: 'closeToTray', title: 'Close to Tray', desc: 'Minimize window to system tray on close' },
                      { key: 'autostart', title: 'Autostart with Windows', desc: 'Launch minimized on user login' },
                      { key: 'wifiSharing', title: 'LAN Hotspot Sharing', desc: 'Bind to 0.0.0.0 for device sharing on local network' },
                    ].map(({ key, title, desc }) => (
                      <div key={key} className="switch-container">
                        <div className="switch-details"><span className="switch-title">{title}</span><span className="switch-desc">{desc}</span></div>
                        <Toggle checked={(settings as any)[key]} onChange={(v) => handleSaveSettings({ [key]: v } as any)} />
                      </div>
                    ))}
                    <div className="switch-container">
                      <div className="switch-details">
                        <span className="switch-title">Virtual TUN Interface</span>
                        <span className="switch-desc">Route all PC traffic natively. Requires Administrator elevation.</span>
                      </div>
                      <Toggle checked={settings.proxyMode === 'tun'} disabled={!isElevated && settings.proxyMode !== 'tun'}
                        onChange={(v) => handleSaveSettings({ proxyMode: v ? 'tun' : 'system' })} />
                    </div>
                  </div>

                  <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                    <SectionLabel>Inbound Ports</SectionLabel>
                    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px' }}>
                      {[
                        { label: 'HTTP Proxy Port', key: 'httpPort', get: httpPort, set: setHttpPort },
                        { label: 'SOCKS5 Port', key: 'socksPort', get: socksPort, set: setSocksPort },
                      ].map(({ label, key, get, set }) => (
                        <div key={key} className="form-group">
                          <div className="flex-row-between">
                            <label className="form-label">{label}</label>
                            <span style={{ fontSize: '11px', fontWeight: 600, color: conflictingPorts.includes(get) ? 'var(--status-error)' : 'var(--status-connected)' }}>
                              {conflictingPorts.includes(get) ? '● Conflict' : '● Available'}
                            </span>
                          </div>
                          <input type="number" className="text-input" style={{ borderColor: conflictingPorts.includes(get) ? 'var(--status-error)' : undefined }}
                            value={get} onChange={(e) => { const v = parseInt(e.target.value) || 0; set(v); handleSaveSettings({ [key]: v } as any); }} />
                        </div>
                      ))}
                    </div>
                    <div className="form-group">
                      <div className="flex-row-between">
                        <label className="form-label">Mixed Inbound Port (Recommended)</label>
                        <span style={{ fontSize: '11px', fontWeight: 600, color: conflictingPorts.includes(mixedPort) ? 'var(--status-error)' : 'var(--status-connected)' }}>
                          {conflictingPorts.includes(mixedPort) ? '● Conflict' : '● Available'}
                        </span>
                      </div>
                      <input type="number" className="text-input" style={{ borderColor: conflictingPorts.includes(mixedPort) ? 'var(--status-error)' : undefined }}
                        value={mixedPort} onChange={(e) => { const v = parseInt(e.target.value) || 0; setMixedPort(v); handleSaveSettings({ mixedPort: v }); }} />
                    </div>
                    <p style={{ fontSize: '12px', color: 'var(--text-muted)', lineHeight: 1.5 }}>
                      Port changes take effect on next connection restart.
                    </p>
                  </div>
                </div>

                <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                    <Shield size={20} style={{ color: 'var(--accent-cyan)' }} />
                    <SectionLabel>SNI Bypass &amp; Legacy DNS</SectionLabel>
                  </div>
                  <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px' }}>
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">SNI Bypass Hostname</label>
                      <Inp value={settings.sniHost} onChange={(v) => handleSaveSettings({ sniHost: v })} placeholder="aka.ms" />
                      <span style={{ fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px', display: 'block', lineHeight: 1.4 }}>
                        Spoofs TLS SNI to bypass ISP throttling or leverage zero-rated packages.
                      </span>
                    </div>
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">Direct DNS Server (Router/ISP)</label>
                      <Inp value={settings.dnsAddress} onChange={(v) => handleSaveSettings({ dnsAddress: v })} placeholder="Auto (system DNS)" />
                      <span style={{ fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px', display: 'block', lineHeight: 1.4 }}>
                        Override DNS for direct traffic. Leave empty to use system DNS.
                      </span>
                    </div>
                  </div>
                </div>

                <div className="glass-panel" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '24px 30px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '20px' }}>
                    <div className="metric-icon-box" style={{ width: '56px', height: '56px', borderRadius: '14px', background: 'rgba(168,85,247,0.05)', color: 'var(--accent-purple)', borderColor: 'rgba(168,85,247,0.15)' }}>
                      <Cpu size={24} />
                    </div>
                    <div>
                      <h4 style={{ fontSize: '16px', fontWeight: 600 }}>TunX Engine</h4>
                      <p style={{ fontSize: '13px', color: 'var(--text-secondary)', marginTop: '2px' }}>
                        sing-box: <strong style={{ color: 'var(--text-primary)' }}>{singboxVersion.split('\n')[0]}</strong>
                      </p>
                    </div>
                  </div>
                  <div style={{ textAlign: 'right', fontSize: '13px', color: 'var(--text-secondary)' }}>
                    <span>App Version: <strong>v{appVersion}</strong></span>
                  </div>
                </div>
              </div>
            )}

            {/* ── TUN Mode ── */}
            {settingsTab === 'tun' && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
                {!isElevated && (
                  <div className="alert-box warn" style={{ padding: '14px 16px' }}>
                    <AlertTriangle size={16} />
                    <span>TUN Mode requires Administrator elevation. Click "Run as Admin" on the Dashboard.</span>
                  </div>
                )}
                <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '4px' }}>
                    <Network size={18} style={{ color: 'var(--accent-cyan)' }} />
                    <SectionLabel>TUN Interface Configuration</SectionLabel>
                  </div>

                  <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px' }}>
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">TUN Stack Implementation</label>
                      <Sel value={settings.tunStack} onChange={(v) => handleSaveSettings({ tunStack: v as any })} options={[
                        { value: 'mixed', label: 'Mixed (gVisor TCP + system UDP)' },
                        { value: 'gvisor', label: 'gVisor (Full userspace)' },
                        { value: 'system', label: 'System (Kernel-level)' },
                      ]} />
                      <span style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '4px', display: 'block' }}>
                        Mixed: best compatibility. gVisor: best isolation. System: best performance.
                      </span>
                    </div>
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">TUN MTU</label>
                      <input type="number" className="text-input" value={settings.tunMtu} onChange={(e) => handleSaveSettings({ tunMtu: parseInt(e.target.value) || 9000 })} />
                      <span style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '4px', display: 'block' }}>9000 recommended for modern networks. Use 1500 for strict environments.</span>
                    </div>
                  </div>

                  {[
                    { key: 'tunAutoRoute', title: 'Auto Route', desc: 'Automatically add routes to direct traffic into the TUN interface' },
                    { key: 'tunAutoRedirect', title: 'Auto Redirect (nftables)', desc: 'Use nftables-based redirect for transparent proxying on Linux' },
                    { key: 'tunStrictRoute', title: 'Strict Route', desc: 'Block traffic not matching any route to prevent leaks' },
                    { key: 'tunEndpointIndependentNat', title: 'Endpoint Independent NAT', desc: 'Improves UDP NAT traversal (P2P, gaming). Uses more resources.' },
                  ].map(({ key, title, desc }) => (
                    <div key={key} className="switch-container">
                      <div className="switch-details"><span className="switch-title">{title}</span><span className="switch-desc">{desc}</span></div>
                      <Toggle checked={(settings as any)[key]} onChange={(v) => handleSaveSettings({ [key]: v } as any)} disabled={!isElevated} />
                    </div>
                  ))}
                </div>

                <div className="info-box" style={{ padding: '14px 16px' }}>
                  <Info size={14} />
                  <span>TUN mode captures ALL system traffic at the network layer. Combined with Auto Route + Strict Route, it provides full traffic control and DNS leak prevention.</span>
                </div>
              </div>
            )}

            {/* ── Sniffing ── */}
            {settingsTab === 'sniff' && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
                <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '4px' }}>
                    <Eye size={18} style={{ color: 'var(--accent-cyan)' }} />
                    <SectionLabel>Traffic Sniffing &amp; Protocol Detection</SectionLabel>
                  </div>
                  <div className="switch-container" style={{ background: 'rgba(102,252,241,0.03)', borderColor: 'rgba(102,252,241,0.2)' }}>
                    <div className="switch-details">
                      <span className="switch-title">Enable Traffic Sniffing</span>
                      <span className="switch-desc">Inspect connection metadata to extract domain names for routing decisions</span>
                    </div>
                    <Toggle checked={settings.sniffEnabled} onChange={(v) => handleSaveSettings({ sniffEnabled: v })} />
                  </div>

                  <div style={{ opacity: settings.sniffEnabled ? 1 : 0.4, pointerEvents: settings.sniffEnabled ? 'auto' : 'none' }}>
                    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px' }}>
                      {[
                        { key: 'sniffHttp', title: 'HTTP Sniffing', desc: 'Extract hostname from HTTP Host header for domain routing' },
                        { key: 'sniffTls', title: 'TLS/HTTPS Sniffing', desc: 'Extract SNI from TLS ClientHello without decryption' },
                        { key: 'sniffQuic', title: 'QUIC/HTTP3 Sniffing', desc: 'Extract SNI from QUIC Initial packets for HTTP/3 routing' },
                        { key: 'sniffOverrideDestination', title: 'Override Destination', desc: 'Use sniffed domain to override the connection target IP for routing' },
                      ].map(({ key, title, desc }) => (
                        <div key={key} className="switch-container">
                          <div className="switch-details"><span className="switch-title">{title}</span><span className="switch-desc">{desc}</span></div>
                          <Toggle checked={(settings as any)[key]} onChange={(v) => handleSaveSettings({ [key]: v } as any)} />
                        </div>
                      ))}
                    </div>
                  </div>
                </div>

                <div className="info-box" style={{ padding: '14px 16px' }}>
                  <Info size={14} />
                  <span>Sniffing extracts domain metadata from live connections for rule-based routing without decrypting traffic. Recommended: enable HTTP + TLS sniffing with Override Destination for best routing accuracy.</span>
                </div>
              </div>
            )}

            {/* ── Multiplexing ── */}
            {settingsTab === 'mux' && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
                <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '4px' }}>
                    <Layers2 size={18} style={{ color: 'var(--accent-cyan)' }} />
                    <SectionLabel>Connection Multiplexing</SectionLabel>
                  </div>
                  <div className="switch-container" style={{ background: 'rgba(102,252,241,0.03)', borderColor: 'rgba(102,252,241,0.2)' }}>
                    <div className="switch-details">
                      <span className="switch-title">Enable Multiplexing (Mux)</span>
                      <span className="switch-desc">Bundle multiple streams into a single connection to reduce TLS handshake overhead</span>
                    </div>
                    <Toggle checked={settings.muxEnabled} onChange={(v) => handleSaveSettings({ muxEnabled: v })} />
                  </div>

                  <div style={{ opacity: settings.muxEnabled ? 1 : 0.4, pointerEvents: settings.muxEnabled ? 'auto' : 'none' }}>
                    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px', marginBottom: '16px' }}>
                      <div className="form-group" style={{ marginBottom: 0 }}>
                        <label className="form-label">Mux Protocol</label>
                        <Sel value={settings.muxProtocol} onChange={(v) => handleSaveSettings({ muxProtocol: v as any })} options={[
                          { value: 'h2mux', label: 'h2mux (HTTP/2 based, recommended)' },
                          { value: 'smux', label: 'smux (Simple stream mux)' },
                          { value: 'yamux', label: 'yamux (HashiCorp Yamux)' },
                        ]} />
                      </div>
                      <div className="form-group" style={{ marginBottom: 0 }}>
                        <label className="form-label">Max Connections</label>
                        <input type="number" className="text-input" value={settings.muxMaxConnections} onChange={(e) => handleSaveSettings({ muxMaxConnections: parseInt(e.target.value) || 4 })} />
                      </div>
                      <div className="form-group" style={{ marginBottom: 0 }}>
                        <label className="form-label">Min Streams (per connection)</label>
                        <input type="number" className="text-input" value={settings.muxMinStreams} onChange={(e) => handleSaveSettings({ muxMinStreams: parseInt(e.target.value) || 4 })} />
                      </div>
                      <div className="form-group" style={{ marginBottom: 0 }}>
                        <label className="form-label">Max Streams (0 = unlimited)</label>
                        <input type="number" className="text-input" value={settings.muxMaxStreams} onChange={(e) => handleSaveSettings({ muxMaxStreams: parseInt(e.target.value) || 0 })} />
                      </div>
                    </div>
                    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px' }}>
                      <div className="switch-container">
                        <div className="switch-details"><span className="switch-title">Stream Padding</span><span className="switch-desc">Add random padding bytes to obfuscate stream lengths</span></div>
                        <Toggle checked={settings.muxPadding} onChange={(v) => handleSaveSettings({ muxPadding: v })} />
                      </div>
                      <div className="switch-container">
                        <div className="switch-details"><span className="switch-title">Brutal Congestion Control</span><span className="switch-desc">Use fixed bandwidth mode instead of BBR (for limited networks)</span></div>
                        <Toggle checked={settings.muxBrutal} onChange={(v) => handleSaveSettings({ muxBrutal: v })} />
                      </div>
                    </div>
                    {settings.muxBrutal && (
                      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px', marginTop: '16px' }}>
                        <div className="form-group" style={{ marginBottom: 0 }}>
                          <label className="form-label">Brutal Upload Speed (Mbps)</label>
                          <input type="number" className="text-input" value={settings.muxBrutalUpMbps} onChange={(e) => handleSaveSettings({ muxBrutalUpMbps: parseFloat(e.target.value) || 100 })} />
                        </div>
                        <div className="form-group" style={{ marginBottom: 0 }}>
                          <label className="form-label">Brutal Download Speed (Mbps)</label>
                          <input type="number" className="text-input" value={settings.muxBrutalDownMbps} onChange={(e) => handleSaveSettings({ muxBrutalDownMbps: parseFloat(e.target.value) || 100 })} />
                        </div>
                      </div>
                    )}
                  </div>
                </div>
              </div>
            )}

            {/* ── API ── */}
            {settingsTab === 'api' && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
                <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '4px' }}>
                    <Code2 size={18} style={{ color: 'var(--accent-cyan)' }} />
                    <SectionLabel>Clash-Compatible REST API</SectionLabel>
                  </div>
                  <div className="switch-container" style={{ background: 'rgba(102,252,241,0.03)', borderColor: 'rgba(102,252,241,0.2)' }}>
                    <div className="switch-details">
                      <span className="switch-title">Enable External API</span>
                      <span className="switch-desc">Expose a Clash-compatible HTTP API for external controllers (Yacd, MetaCubeX)</span>
                    </div>
                    <Toggle checked={settings.apiEnabled} onChange={(v) => handleSaveSettings({ apiEnabled: v })} />
                  </div>

                  <div style={{ opacity: settings.apiEnabled ? 1 : 0.4, pointerEvents: settings.apiEnabled ? 'auto' : 'none' }}>
                    <div style={{ display: 'grid', gridTemplateColumns: '160px 1fr', gap: '16px', marginBottom: '16px' }}>
                      <div className="form-group" style={{ marginBottom: 0 }}>
                        <label className="form-label">API Port</label>
                        <input type="number" className="text-input" value={settings.apiPort} onChange={(e) => handleSaveSettings({ apiPort: parseInt(e.target.value) || 9090 })} />
                      </div>
                      <div className="form-group" style={{ marginBottom: 0 }}>
                        <label className="form-label">API Secret Token</label>
                        <div style={{ position: 'relative' }}>
                          <Inp value={settings.apiSecret} onChange={(v) => handleSaveSettings({ apiSecret: v })} placeholder="Leave empty for no auth" mono />
                        </div>
                      </div>
                    </div>
                    <div className="switch-container">
                      <div className="switch-details"><span className="switch-title">Allow CORS (Cross-Origin)</span><span className="switch-desc">Allow web-based controllers (Yacd-meta, Metacubexd) to connect from browser</span></div>
                      <Toggle checked={settings.apiCors} onChange={(v) => handleSaveSettings({ apiCors: v })} />
                    </div>
                    {settings.apiEnabled && isConnected && (
                      <div className="info-box success" style={{ marginTop: '16px', background: 'rgba(16,185,129,0.06)', borderColor: 'rgba(16,185,129,0.2)', color: 'var(--status-connected)' }}>
                        <CheckCircle2 size={14} />
                        <span>API active at <code>http://127.0.0.1:{settings.apiPort}</code> — open in <a href={`http://yacd.haishan.me/?hostname=127.0.0.1&port=${settings.apiPort}&secret=${settings.apiSecret}`} target="_blank" rel="noreferrer" style={{ color: 'var(--accent-cyan)', textDecoration: 'underline' }}>Yacd UI <ExternalLink size={11} /></a></span>
                      </div>
                    )}
                  </div>
                </div>

                <div className="info-box" style={{ padding: '14px 16px' }}>
                  <Info size={14} />
                  <span>The Clash-compatible API lets external tools like Yacd or MetaCubeX dashboard control routing, switch proxies, and monitor connections in real-time. Requires sing-box experimental.clash_api to be compiled in.</span>
                </div>
              </div>
            )}
          </div>
        )}
      </main>

      {/* ═══════════════════════════════════════════════════════════════════════
          NODE CONFIGURATION EDITOR MODAL (EXPANDED — ALL PROTOCOLS)
      ═══════════════════════════════════════════════════════════════════════ */}
      {isEditingConfig && (
        <div className="editor-overlay">
          <div className="editor-modal">
            <header className="editor-header">
              <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
                <span className="editor-title">Edit Outbound Node</span>
                <span className="type-chip" style={{ fontFamily: 'var(--font-mono)', fontSize: '11px' }}>
                  {PROTOCOL_LABELS[editProtocol] || editProtocol}
                </span>
              </div>
              <button className="btn-icon-only" onClick={() => setIsEditingConfig(false)} style={{ border: 'none', background: 'transparent' }}>
                <X size={18} style={{ color: 'var(--text-secondary)' }} />
              </button>
            </header>

            {/* Editor sub-tabs */}
            {!['wireguard','ssh','socks','http'].includes(editProtocol) && (
              <div style={{ display: 'flex', gap: '4px', padding: '0 24px', borderBottom: '1px solid var(--border-glass)' }}>
                {[['basic','Basic'], ['transport','Transport'], ['tls','TLS / Security']].map(([id, label]) => (
                  <button key={id} onClick={() => setEditorSection(id as any)}
                    style={{ padding: '10px 16px', background: 'transparent', border: 'none', cursor: 'pointer', fontSize: '13px', fontWeight: 500, color: editorSection === id ? 'var(--accent-cyan)' : 'var(--text-secondary)', borderBottom: editorSection === id ? '2px solid var(--accent-cyan)' : '2px solid transparent', transition: 'all 0.2s' }}>
                    {label}
                  </button>
                ))}
              </div>
            )}

            <div className="editor-body">
              {/* ── BASIC (all protocols) ── */}
              {(editorSection === 'basic' || ['wireguard','ssh','socks','http'].includes(editProtocol)) && (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
                  {/* Protocol selector + common fields */}
                  <SubCard title="Common">
                    <div className="editor-form-grid-2">
                      <div className="form-group">
                        <label className="form-label">Node Name (Tag)*</label>
                        <Inp value={editTag} onChange={setEditTag} placeholder="my-proxy-sg" />
                      </div>
                      <div className="form-group">
                        <label className="form-label">Protocol Type</label>
                        <Sel value={editProtocol} onChange={(v) => { setEditProtocol(v); setEditorSection('basic'); }} options={[
                          { value: 'vless', label: 'VLESS' }, { value: 'vmess', label: 'VMess' },
                          { value: 'trojan', label: 'Trojan' }, { value: 'shadowsocks', label: 'Shadowsocks' },
                          { value: 'hysteria2', label: 'Hysteria2' }, { value: 'tuic', label: 'TUIC v5' },
                          { value: 'wireguard', label: 'WireGuard' }, { value: 'ssh', label: 'SSH' },
                          { value: 'socks', label: 'SOCKS5' }, { value: 'http', label: 'HTTP Proxy' },
                        ]} />
                      </div>
                    </div>
                    {!['wireguard'].includes(editProtocol) && (
                      <div className="editor-form-grid-2">
                        <div className="form-group">
                          <label className="form-label">Server Address*</label>
                          <Inp value={editAddress} onChange={setEditAddress} placeholder="sg.example.com or 1.2.3.4" mono />
                        </div>
                        <div className="form-group">
                          <label className="form-label">Port*</label>
                          <input type="number" className="text-input" value={editPort} onChange={(e) => setEditPort(Number(e.target.value) || 443)} />
                        </div>
                      </div>
                    )}
                  </SubCard>

                  {/* VLESS/VMess */}
                  {(editProtocol === 'vless' || editProtocol === 'vmess') && (
                    <SubCard title="VLESS / VMess Authentication">
                      <div className="form-group">
                        <label className="form-label">UUID*</label>
                        <Inp value={editUuid} onChange={setEditUuid} placeholder="a20d4836-4634-471b-b6c3-2f48acb0e9a5" mono />
                      </div>
                      {editProtocol === 'vless' && (
                        <div className="form-group">
                          <label className="form-label">Flow (XTLS Vision)</label>
                          <Sel value={editFlow} onChange={setEditFlow} options={[
                            { value: '', label: 'None (Default)' },
                            { value: 'xtls-rprx-vision', label: 'xtls-rprx-vision' },
                          ]} />
                        </div>
                      )}
                      {editProtocol === 'vmess' && (
                        <div className="form-group">
                          <label className="form-label">Security</label>
                          <Sel value={editMethod || 'auto'} onChange={setEditMethod} options={[
                            { value: 'auto', label: 'Auto' }, { value: 'aes-128-gcm', label: 'AES-128-GCM' },
                            { value: 'chacha20-poly1305', label: 'ChaCha20-Poly1305' }, { value: 'none', label: 'None (Plain)' },
                          ]} />
                        </div>
                      )}
                    </SubCard>
                  )}

                  {/* Trojan */}
                  {editProtocol === 'trojan' && (
                    <SubCard title="Trojan Authentication">
                      <div className="form-group">
                        <label className="form-label">Password*</label>
                        <input type="password" className="text-input" value={editPassword} onChange={(e) => setEditPassword(e.target.value)} placeholder="Your Trojan password" />
                      </div>
                    </SubCard>
                  )}

                  {/* Shadowsocks */}
                  {editProtocol === 'shadowsocks' && (
                    <SubCard title="Shadowsocks Authentication">
                      <div className="editor-form-grid-2">
                        <div className="form-group">
                          <label className="form-label">Password*</label>
                          <input type="password" className="text-input" value={editPassword} onChange={(e) => setEditPassword(e.target.value)} placeholder="Password" />
                        </div>
                        <div className="form-group">
                          <label className="form-label">Encryption Method*</label>
                          <Sel value={editMethod} onChange={setEditMethod} options={[
                            { value: 'aes-256-gcm', label: 'AES-256-GCM' },
                            { value: 'aes-128-gcm', label: 'AES-128-GCM' },
                            { value: 'chacha20-ietf-poly1305', label: 'ChaCha20-Poly1305' },
                            { value: '2022-blake3-aes-256-gcm', label: '2022-Blake3-AES-256-GCM' },
                            { value: '2022-blake3-aes-128-gcm', label: '2022-Blake3-AES-128-GCM' },
                            { value: '2022-blake3-chacha20-poly1305', label: '2022-Blake3-ChaCha20-Poly1305' },
                          ]} />
                        </div>
                      </div>
                    </SubCard>
                  )}

                  {/* Hysteria2 */}
                  {editProtocol === 'hysteria2' && (
                    <>
                      <SubCard title="Hysteria2 Authentication">
                        <div className="form-group">
                          <label className="form-label">Auth Password*</label>
                          <input type="password" className="text-input" value={editHy2Auth} onChange={(e) => setEditHy2Auth(e.target.value)} placeholder="Your Hysteria2 password" />
                        </div>
                        <div className="editor-form-grid-2">
                          <div className="form-group">
                            <label className="form-label">Upload Bandwidth</label>
                            <Inp value={editHy2UpBw} onChange={setEditHy2UpBw} placeholder="100 mbps" mono />
                          </div>
                          <div className="form-group">
                            <label className="form-label">Download Bandwidth</label>
                            <Inp value={editHy2DownBw} onChange={setEditHy2DownBw} placeholder="200 mbps" mono />
                          </div>
                        </div>
                        <span style={{ fontSize: '11px', color: 'var(--text-muted)', lineHeight: 1.5, display: 'block' }}>
                          Supported units: bps, kbps, mbps, gbps. Use your actual ISP bandwidth for best performance.
                        </span>
                      </SubCard>
                      <SubCard title="Hysteria2 Obfuscation (Optional)">
                        <div className="editor-form-grid-2">
                          <div className="form-group">
                            <label className="form-label">Obfuscation Type</label>
                            <Sel value={editHy2ObfsType} onChange={setEditHy2ObfsType} options={[
                              { value: '', label: 'None' }, { value: 'salamander', label: 'Salamander' },
                            ]} />
                          </div>
                          {editHy2ObfsType && (
                            <div className="form-group">
                              <label className="form-label">Obfuscation Password</label>
                              <input type="password" className="text-input" value={editHy2ObfsPassword} onChange={(e) => setEditHy2ObfsPassword(e.target.value)} placeholder="Obfuscation key" />
                            </div>
                          )}
                        </div>
                      </SubCard>
                      <SubCard title="TLS (Hysteria2 requires TLS)">
                        <div className="editor-form-grid-2">
                          <div className="form-group">
                            <label className="form-label">SNI</label>
                            <Inp value={editServerName} onChange={setEditServerName} placeholder="your-server.example.com" />
                          </div>
                          <div className="form-group">
                            <label className="form-label">ALPN</label>
                            <Inp value={editAlpn} onChange={setEditAlpn} placeholder="h3" mono />
                          </div>
                        </div>
                        <div className="editor-checkbox-row">
                          <input type="checkbox" id="hy2insecure" checked={editAllowInsecure} onChange={(e) => setEditAllowInsecure(e.target.checked)} />
                          <label htmlFor="hy2insecure" className="editor-checkbox-label">Allow Insecure (skip cert verification)</label>
                        </div>
                      </SubCard>
                    </>
                  )}

                  {/* TUIC */}
                  {editProtocol === 'tuic' && (
                    <>
                      <SubCard title="TUIC v5 Authentication">
                        <div className="editor-form-grid-2">
                          <div className="form-group">
                            <label className="form-label">UUID*</label>
                            <Inp value={editTuicUuid} onChange={setEditTuicUuid} placeholder="UUID from server" mono />
                          </div>
                          <div className="form-group">
                            <label className="form-label">Password*</label>
                            <input type="password" className="text-input" value={editTuicPassword} onChange={(e) => setEditTuicPassword(e.target.value)} placeholder="TUIC password" />
                          </div>
                        </div>
                      </SubCard>
                      <SubCard title="TUIC Performance">
                        <div className="editor-form-grid-2">
                          <div className="form-group">
                            <label className="form-label">Congestion Control</label>
                            <Sel value={editTuicCongestion} onChange={setEditTuicCongestion} options={[
                              { value: 'bbr', label: 'BBR (Recommended)' },
                              { value: 'cubic', label: 'CUBIC' },
                              { value: 'new_reno', label: 'New Reno' },
                            ]} />
                          </div>
                          <div className="form-group">
                            <label className="form-label">UDP Relay Mode</label>
                            <Sel value={editTuicUdpMode} onChange={setEditTuicUdpMode} options={[
                              { value: 'native', label: 'Native (direct UDP)' },
                              { value: 'quic', label: 'QUIC (tunneled)' },
                            ]} />
                          </div>
                        </div>
                      </SubCard>
                      <SubCard title="TLS (TUIC requires TLS)">
                        <div className="editor-form-grid-2">
                          <div className="form-group">
                            <label className="form-label">SNI</label>
                            <Inp value={editServerName} onChange={setEditServerName} placeholder="your-server.example.com" />
                          </div>
                          <div className="form-group">
                            <label className="form-label">ALPN</label>
                            <Inp value={editAlpn} onChange={setEditAlpn} placeholder="h3" mono />
                          </div>
                        </div>
                        <div className="editor-checkbox-row">
                          <input type="checkbox" id="tuicinsecure" checked={editAllowInsecure} onChange={(e) => setEditAllowInsecure(e.target.checked)} />
                          <label htmlFor="tuicinsecure" className="editor-checkbox-label">Allow Insecure</label>
                        </div>
                      </SubCard>
                    </>
                  )}

                  {/* WireGuard */}
                  {editProtocol === 'wireguard' && (
                    <>
                      <SubCard title="WireGuard Keys">
                        <div className="form-group">
                          <label className="form-label">Client Private Key*</label>
                          <Inp value={editWgSecretKey} onChange={setEditWgSecretKey} placeholder="Base64 encoded private key" mono />
                        </div>
                      </SubCard>
                      <SubCard title="Peer Configuration">
                        <div className="form-group">
                          <label className="form-label">Server Endpoint (host:port)*</label>
                          <Inp value={editWgEndpoint} onChange={setEditWgEndpoint} placeholder="wg.example.com:51820" mono />
                        </div>
                        <div className="form-group">
                          <label className="form-label">Peer Public Key*</label>
                          <Inp value={editWgPeerPublicKey} onChange={setEditWgPeerPublicKey} placeholder="Base64 peer public key" mono />
                        </div>
                        <div className="form-group">
                          <label className="form-label">Pre-Shared Key (optional)</label>
                          <Inp value={editWgPreSharedKey} onChange={setEditWgPreSharedKey} placeholder="Optional PSK for extra security" mono />
                        </div>
                        <div className="form-group">
                          <label className="form-label">Allowed IPs</label>
                          <Inp value={editWgAllowedIps} onChange={setEditWgAllowedIps} placeholder="0.0.0.0/0, ::/0" mono />
                        </div>
                      </SubCard>
                      <SubCard title="Advanced WireGuard">
                        <div className="editor-form-grid-2">
                          <div className="form-group">
                            <label className="form-label">Reserved Bytes (comma separated)</label>
                            <Inp value={editWgReserved} onChange={setEditWgReserved} placeholder="0,0,0 (e.g. for Cloudflare WARP)" mono />
                          </div>
                          <div className="form-group">
                            <label className="form-label">MTU</label>
                            <input type="number" className="text-input" value={editWgMtu} onChange={(e) => setEditWgMtu(parseInt(e.target.value) || 1280)} />
                          </div>
                        </div>
                      </SubCard>
                    </>
                  )}

                  {/* SSH */}
                  {editProtocol === 'ssh' && (
                    <>
                      <SubCard title="SSH Authentication">
                        <div className="form-group">
                          <label className="form-label">Username*</label>
                          <Inp value={editSshUser} onChange={setEditSshUser} placeholder="root" mono />
                        </div>
                        <div className="form-group">
                          <label className="form-label">Password (optional)</label>
                          <input type="password" className="text-input" value={editSshPassword} onChange={(e) => setEditSshPassword(e.target.value)} placeholder="Leave empty to use key auth" />
                        </div>
                        <div className="form-group">
                          <label className="form-label">Private Key (PEM format)</label>
                          <textarea className="text-input" style={{ height: '80px', fontFamily: 'var(--font-mono)', fontSize: '11px', resize: 'vertical' }}
                            value={editSshPrivateKey} onChange={(e) => setEditSshPrivateKey(e.target.value)}
                            placeholder="-----BEGIN OPENSSH PRIVATE KEY-----&#10;...&#10;-----END OPENSSH PRIVATE KEY-----" />
                        </div>
                      </SubCard>
                      <SubCard title="SSH Host Key Verification">
                        <div className="form-group">
                          <label className="form-label">Host Key (Base64, leave empty to skip verification)</label>
                          <Inp value={editSshHostKey} onChange={setEditSshHostKey} placeholder="AAAAE2VjZHNhLXNoYTItbmlzdHAy..." mono />
                        </div>
                      </SubCard>
                    </>
                  )}

                  {/* SOCKS5 */}
                  {editProtocol === 'socks' && (
                    <SubCard title="SOCKS Configuration">
                      <div className="editor-form-grid-2">
                        <div className="form-group">
                          <label className="form-label">SOCKS Version</label>
                          <Sel value={String(editSocksVersion)} onChange={(v) => setEditSocksVersion(parseInt(v))} options={[
                            { value: '5', label: 'SOCKS5' }, { value: '4', label: 'SOCKS4' }, { value: '4a', label: 'SOCKS4a' },
                          ]} />
                        </div>
                        <div className="form-group">
                          <label className="form-label">Username</label>
                          <Inp value={editSocksUser} onChange={setEditSocksUser} placeholder="Optional" />
                        </div>
                        <div className="form-group">
                          <label className="form-label">Password</label>
                          <input type="password" className="text-input" value={editSocksPassword} onChange={(e) => setEditSocksPassword(e.target.value)} placeholder="Optional" />
                        </div>
                      </div>
                      <div className="switch-container" style={{ padding: '10px 14px', marginTop: '4px' }}>
                        <div className="switch-details"><span className="switch-title" style={{ fontSize: '13px' }}>UDP over TCP</span><span className="switch-desc">Tunnel UDP packets over the TCP SOCKS connection</span></div>
                        <Toggle checked={editSocksUdpOverTcp} onChange={setEditSocksUdpOverTcp} />
                      </div>
                    </SubCard>
                  )}

                  {/* HTTP outbound */}
                  {editProtocol === 'http' && (
                    <SubCard title="HTTP Proxy Configuration">
                      <div className="editor-form-grid-2">
                        <div className="form-group">
                          <label className="form-label">Username</label>
                          <Inp value={editHttpUser} onChange={setEditHttpUser} placeholder="Optional" />
                        </div>
                        <div className="form-group">
                          <label className="form-label">Password</label>
                          <input type="password" className="text-input" value={editHttpPassword} onChange={(e) => setEditHttpPassword(e.target.value)} placeholder="Optional" />
                        </div>
                      </div>
                      <div className="switch-container" style={{ padding: '10px 14px' }}>
                        <div className="switch-details"><span className="switch-title" style={{ fontSize: '13px' }}>HTTPS (TLS)</span><span className="switch-desc">Connect to HTTPS CONNECT proxy instead of plain HTTP</span></div>
                        <Toggle checked={editHttpTls} onChange={setEditHttpTls} />
                      </div>
                    </SubCard>
                  )}
                </div>
              )}

              {/* ── TRANSPORT ── */}
              {editorSection === 'transport' && !['wireguard','ssh','socks','http','hysteria2','tuic'].includes(editProtocol) && (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
                  <SubCard title="Transport Layer">
                    <div className="editor-form-grid-2">
                      <div className="form-group">
                        <label className="form-label">Network Transport</label>
                        <Sel value={editNetwork} onChange={(v) => setEditNetwork(v as any)} options={[
                          { value: 'tcp', label: 'TCP (Raw)' }, { value: 'ws', label: 'WebSocket (WS)' },
                          { value: 'grpc', label: 'gRPC' }, { value: 'http', label: 'HTTP/2 (h2)' },
                          { value: 'httpupgrade', label: 'HTTP Upgrade' }, { value: 'quic', label: 'QUIC' },
                        ]} />
                      </div>
                      {editNetwork === 'tcp' && (
                        <div className="form-group">
                          <label className="form-label">TCP Header Camouflage</label>
                          <Sel value={editHeaderType} onChange={setEditHeaderType} options={[
                            { value: '', label: 'None' }, { value: 'http', label: 'HTTP (HTTP Masquerade)' },
                          ]} />
                        </div>
                      )}
                    </div>
                  </SubCard>

                  {(editNetwork === 'ws' || editNetwork === 'http' || editNetwork === 'httpupgrade' || editHeaderType === 'http') && (
                    <SubCard title="HTTP / WebSocket Settings">
                      <div className="editor-form-grid-2">
                        <div className="form-group">
                          <label className="form-label">Path*</label>
                          <Inp value={editPath} onChange={setEditPath} placeholder="/graphql" mono />
                        </div>
                        <div className="form-group">
                          <label className="form-label">Host Header</label>
                          <Inp value={editHost} onChange={setEditHost} placeholder="cdn.example.com" mono />
                        </div>
                      </div>
                      {editNetwork === 'ws' && (
                        <div className="form-group" style={{ marginBottom: 0 }}>
                          <label className="form-label">Early Data Header (optional, 0rtt)</label>
                          <Inp value={editHeaderType} onChange={setEditHeaderType} placeholder="Sec-WebSocket-Protocol" mono />
                        </div>
                      )}
                    </SubCard>
                  )}

                  {editNetwork === 'grpc' && (
                    <SubCard title="gRPC Settings">
                      <div className="form-group">
                        <label className="form-label">Service Name*</label>
                        <Inp value={editServiceName} onChange={setEditServiceName} placeholder="GunService" mono />
                      </div>
                    </SubCard>
                  )}
                </div>
              )}

              {/* ── TLS ── */}
              {editorSection === 'tls' && !['wireguard','ssh','socks','http','hysteria2','tuic'].includes(editProtocol) && (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
                  <SubCard title="TLS Security">
                    <div className="editor-form-grid-2">
                      <div className="form-group">
                        <label className="form-label">Security Mode</label>
                        <Sel value={editTlsEnabled ? (editRealityEnabled ? 'reality' : 'tls') : 'none'} onChange={(v) => {
                          if (v === 'none') { setEditTlsEnabled(false); setEditRealityEnabled(false); }
                          else if (v === 'tls') { setEditTlsEnabled(true); setEditRealityEnabled(false); }
                          else { setEditTlsEnabled(true); setEditRealityEnabled(true); }
                        }} options={[{ value: 'none', label: 'None (Plain)' }, { value: 'tls', label: 'TLS/SSL' }, { value: 'reality', label: 'REALITY (XTLS)' }]} />
                      </div>
                      {editTlsEnabled && (
                        <div className="form-group">
                          <label className="form-label">uTLS Fingerprint</label>
                          <Sel value={editFingerprint} onChange={setEditFingerprint} options={[
                            { value: 'chrome', label: 'Chrome (Latest)' }, { value: 'firefox', label: 'Firefox' },
                            { value: 'edge', label: 'Microsoft Edge' }, { value: 'safari', label: 'Safari' },
                            { value: '360', label: '360 Browser' }, { value: 'qq', label: 'QQ Browser' },
                            { value: 'ios', label: 'iOS' }, { value: 'android', label: 'Android' },
                            { value: 'random', label: 'Random' }, { value: 'randomized', label: 'Randomized (New each time)' },
                          ]} />
                        </div>
                      )}
                    </div>
                  </SubCard>

                  {editTlsEnabled && (
                    <SubCard title="TLS Parameters">
                      <div className="editor-form-grid-2">
                        <div className="form-group">
                          <label className="form-label">SNI (Server Name Indication)</label>
                          <Inp value={editServerName} onChange={setEditServerName} placeholder="aka.ms" mono />
                        </div>
                        <div className="form-group">
                          <label className="form-label">ALPN (comma separated)</label>
                          <Inp value={editAlpn} onChange={setEditAlpn} placeholder="h2, http/1.1" mono />
                        </div>
                      </div>
                      <div className="editor-checkbox-row" style={{ marginTop: '8px' }}>
                        <input type="checkbox" id="allowInsecure" checked={editAllowInsecure} onChange={(e) => setEditAllowInsecure(e.target.checked)} />
                        <label htmlFor="allowInsecure" className="editor-checkbox-label">Allow Insecure (skip certificate verification)</label>
                      </div>
                    </SubCard>
                  )}

                  {editTlsEnabled && editRealityEnabled && (
                    <SubCard title="REALITY Configuration">
                      <div className="info-box" style={{ marginBottom: '14px' }}>
                        <Info size={12} />
                        <span>REALITY is XTLS's anti-censorship protocol that impersonates a real TLS server using its certificate. Requires uTLS fingerprint.</span>
                      </div>
                      <div className="form-group">
                        <label className="form-label">Public Key (pbk)*</label>
                        <Inp value={editPublicKey} onChange={setEditPublicKey} placeholder="kovJ2h9HyJoofO83M00dYx8hzqeTBKyGQ3fs4Y2nMSE" mono />
                      </div>
                      <div className="editor-form-grid-2">
                        <div className="form-group" style={{ marginBottom: 0 }}>
                          <label className="form-label">Short ID (sid)</label>
                          <Inp value={editShortId} onChange={setEditShortId} placeholder="97c4e5fcb1e8" mono />
                        </div>
                        <div className="form-group" style={{ marginBottom: 0 }}>
                          <label className="form-label">Destination SNI</label>
                          <Inp value={editServerName} onChange={setEditServerName} placeholder="aka.ms, itunes.apple.com..." mono />
                        </div>
                      </div>
                    </SubCard>
                  )}
                </div>
              )}

              {editSaveError && (
                <div className="alert-box error" style={{ marginTop: '20px', padding: '12px 16px' }}>
                  <ShieldAlert size={16} /><span>{editSaveError}</span>
                </div>
              )}
            </div>

            <footer className="editor-footer">
              <div className="editor-footer-left">
                <button type="button" className="btn btn-secondary" style={{ fontSize: '12px' }} onClick={() => {
                  setEditTag('zoom-One-Piece'); setEditAddress('azureedge.duckdns.org'); setEditPort(443);
                  setEditProtocol('vless'); setEditUuid('a20d4836-4634-471b-b6c3-2f48acb0e9a5');
                  setEditFlow(''); setEditNetwork('tcp'); setEditTlsEnabled(true); setEditRealityEnabled(true);
                  setEditServerName('aka.ms'); setEditFingerprint('chrome');
                  setEditPublicKey('kovJ2h9HyJoofO83M00dYx8hzqeTBKyGQ3fs4Y2nMSE'); setEditShortId('97c4e5fcb1e8');
                  setEditAllowInsecure(true); setEditorSection('basic');
                  pushSystemLog('Applied VLESS+REALITY preset template.');
                }}>Apply Preset</button>
              </div>
              <div className="editor-footer-right">
                <button type="button" className="btn btn-secondary" onClick={() => setIsEditingConfig(false)}>Cancel</button>
                <button type="button" className="btn btn-primary" onClick={handleSaveEditor} disabled={isSavingConfig} style={{ minWidth: '80px' }}>
                  {isSavingConfig ? <><RefreshCw size={14} className="spin" /> Validating...</> : 'Save Node'}
                </button>
              </div>
            </footer>
          </div>
        </div>
      )}
    </div>
  );
}
