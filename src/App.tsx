import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { readText } from '@tauri-apps/plugin-clipboard-manager';
import {
  Activity,
  Layers,
  Terminal,
  Settings as SettingsIcon,
  Power,
  UploadCloud,
  DownloadCloud,
  Clock,
  Cpu,
  Globe,
  Plus,
  Trash2,
  Check,
  RefreshCw,
  FolderOpen,
  Clipboard,
  Shield,
  ShieldAlert,
  Server,
  Zap,
} from 'lucide-react';
import { TrafficChart } from './components/TrafficChart';
import { storeHelper } from './utils/store';
import type { Profile, Settings } from './utils/store';

// Helper to format bytes (e.g. 10.4 MB, 450 KB)
const formatBytes = (bytes: number): string => {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
};

// Helper to format speed (e.g. 1.2 MB/s, 450 KB/s)
const formatSpeed = (bytesPerSec: number): string => {
  if (bytesPerSec === 0) return '0 B/s';
  const k = 1024;
  const sizes = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
  const i = Math.floor(Math.log(bytesPerSec) / Math.log(k));
  return parseFloat((bytesPerSec / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
};

// Helper to format uptime (e.g. 02h 15m 30s)
const formatUptime = (seconds: number): string => {
  const h = Math.floor(seconds / 3600).toString().padStart(2, '0');
  const m = Math.floor((seconds % 3600) / 60).toString().padStart(2, '0');
  const s = (seconds % 60).toString().padStart(2, '0');
  return `${h}h ${m}m ${s}s`;
};

export default function App() {
  // Navigation State
  const [activeTab, setActiveTab] = useState<'dashboard' | 'profiles' | 'logs' | 'settings'>('dashboard');

  // Tauri / App State
  const [isConnected, setIsConnected] = useState<boolean>(false);
  const [activeProfileId, setActiveProfileId] = useState<string | null>(null);
  const [uptime, setUptime] = useState<number>(0);
  const [httpPort, setHttpPort] = useState<number>(7890);
  const [socksPort, setSocksPort] = useState<number>(7891);
  const [mixedPort, setMixedPort] = useState<number>(7892);
  const [isElevated, setIsElevated] = useState<boolean>(false);
  const [singboxVersion, setSingboxVersion] = useState<string>('Unknown');
  const [appVersion, setAppVersion] = useState<string>('0.1.0');

  // Traffic Stats & History
  const [uploadBytes, setUploadBytes] = useState<number>(0);
  const [downloadBytes, setDownloadBytes] = useState<number>(0);
  const [uploadSpeed, setUploadSpeed] = useState<number>(0);
  const [downloadSpeed, setDownloadSpeed] = useState<number>(0);
  const [activeConnections, setActiveConnections] = useState<number>(0);
  const [speedHistory, setSpeedHistory] = useState<{ up: number; down: number }[]>([]);

  // Profiles State
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [selectedProfileId, setSelectedProfileId] = useState<string | null>(null);
  const [profileOutbounds, setProfileOutbounds] = useState<any[]>([]);
  const [selectedOutboundTag, setSelectedOutboundTag] = useState<string | null>(null);

  // Profile Import Inputs
  const [importTab, setImportTab] = useState<'url' | 'file' | 'clipboard'>('url');
  const [importName, setImportName] = useState<string>('');
  const [importUrl, setImportUrl] = useState<string>('');
  const [importContent, setImportContent] = useState<string>('');
  const [importFilePath, setImportFilePath] = useState<string>('');
  const [importError, setImportError] = useState<string | null>(null);
  const [importSuccess, setImportSuccess] = useState<boolean>(false);
  const [isImporting, setIsImporting] = useState<boolean>(false);

  // Settings State
  const [settings, setSettings] = useState<Settings>({
    proxyMode: 'system',
    closeToTray: true,
    autostart: false,
    httpPort: 7890,
    socksPort: 7891,
    mixedPort: 7892,
    dnsAddress: '',
    sniHost: 'aka.ms',
    wifiSharing: false,
  });

  // Logs State
  const [logs, setLogs] = useState<{ type: 'info' | 'warn' | 'error' | 'system'; text: string }[]>([]);
  const [autoScrollLogs, setAutoScrollLogs] = useState<boolean>(true);
  const consoleEndRef = useRef<HTMLDivElement | null>(null);

  // --- OUTBOUND CONFIGURATION EDITOR STATE ---
  const [isEditingConfig, setIsEditingConfig] = useState<boolean>(false);
  const [editTag, setEditTag] = useState<string>('');
  const [editAddress, setEditAddress] = useState<string>('');
  const [editPort, setEditPort] = useState<number>(443);
  const [editProtocol, setEditProtocol] = useState<string>('vless');
  const [editUuid, setEditUuid] = useState<string>('');
  const [editFlow, setEditFlow] = useState<string>('');
  const [editPassword, setEditPassword] = useState<string>('');
  const [editMethod, setEditMethod] = useState<string>('');
  const [editNetwork, setEditNetwork] = useState<'tcp' | 'ws' | 'grpc'>('tcp');
  const [editHeaderType, setEditHeaderType] = useState<string>('');
  const [editPath, setEditPath] = useState<string>('');
  const [editHost, setEditHost] = useState<string>('');
  const [editTlsEnabled, setEditTlsEnabled] = useState<boolean>(false);
  const [editAllowInsecure, setEditAllowInsecure] = useState<boolean>(false);
  const [editServerName, setEditServerName] = useState<string>('');
  const [editAlpn, setEditAlpn] = useState<string>('');
  const [editRealityEnabled, setEditRealityEnabled] = useState<boolean>(false);
  const [editFingerprint, setEditFingerprint] = useState<string>('');
  const [editPublicKey, setEditPublicKey] = useState<string>('');
  const [editShortId, setEditShortId] = useState<string>('');
  const [editSaveError, setEditSaveError] = useState<string | null>(null);
  const [isSavingConfig, setIsSavingConfig] = useState<boolean>(false);

  // --- PORT CONFLICT STATE ---
  const [conflictingPorts, setConflictingPorts] = useState<number[]>([]);

  // --- INITIALIZATION ---
  useEffect(() => {
    // 1. Fetch system versions and elevation status
    const initSystemInfo = async () => {
      try {
        const hasTun = await invoke<boolean>('check_tun_support');
        setIsElevated(hasTun);

        const verApp = await invoke<string>('get_app_version');
        setAppVersion(verApp);

        const verSing = await invoke<string>('get_singbox_version');
        setSingboxVersion(verSing);
      } catch (e) {
        console.error('Failed to load system info:', e);
      }
    };
    initSystemInfo();

    // 2. Load Profiles and Settings from persistent store
    const initStore = async () => {
      const savedSettings = await storeHelper.getSettings();
      setSettings(savedSettings);
      setHttpPort(savedSettings.httpPort);
      setSocksPort(savedSettings.socksPort);
      setMixedPort(savedSettings.mixedPort);

      const savedProfiles = await storeHelper.getProfiles();
      setProfiles(savedProfiles);

      if (savedProfiles.length > 0) {
        setSelectedProfileId(savedProfiles[0].id);
      }
    };
    initStore();

    // Add initial system logs
    pushSystemLog('TunX Core Engine initialized.');
    pushSystemLog('Ready to establish proxy tunnels.');

    // 3. Listen to live logs emitted by the sing-box sidecar process
    const unlistenLog = listen<string>('sing-box-log', (event) => {
      const line = event.payload.trim();
      if (!line) return;

      let type: 'info' | 'warn' | 'error' | 'system' = 'info';
      if (line.toLowerCase().includes('warn')) {
        type = 'warn';
      } else if (line.toLowerCase().includes('err') || line.toLowerCase().includes('fatal')) {
        type = 'error';
      }
      setLogs((prev) => [...prev, { type, text: line }]);
    });

    // 4. Listen to sidecar premature termination
    const unlistenTerm = listen<number | null>('sing-box-terminated', (event) => {
      setIsConnected(false);
      setActiveProfileId(null);
      pushSystemLog(`sing-box service terminated with code ${event.payload ?? 'None'}`);
    });

    // Load buffered logs initially
    const syncBufferedLogs = async () => {
      try {
        const buffered = await invoke<string[]>('get_buffered_logs');
        if (buffered && buffered.length > 0) {
          const formatted = buffered.map((line) => {
            let type: 'info' | 'warn' | 'error' | 'system' = 'info';
            if (line.toLowerCase().includes('warn')) type = 'warn';
            else if (line.toLowerCase().includes('err')) type = 'error';
            return { type, text: line };
          });
          setLogs((prev) => [...prev, ...formatted]);
        }
      } catch (_) {}
    };
    syncBufferedLogs();

    return () => {
      // Clean up event listeners
      unlistenLog.then((f) => f());
      unlistenTerm.then((f) => f());
    };
  }, []);

  // --- REAL-TIME PORT CONFLICT CHECKER FOR SETTINGS VIEW ---
  useEffect(() => {
    const checkConflicts = async () => {
      const ports = [httpPort, socksPort, mixedPort];
      const conflicts: number[] = [];
      for (const p of ports) {
        try {
          // Check conflict of single port
          const res = await invoke<number | null>('check_port_conflict', { ports: [p] });
          if (res) {
            conflicts.push(p);
          }
        } catch (_) {}
      }
      setConflictingPorts(conflicts);
    };

    checkConflicts();
    // Run again when tab changes or ports change
  }, [activeTab, httpPort, socksPort, mixedPort]);

  // --- REAL-TIME POLL TIMER (1 SECOND) ---
  useEffect(() => {
    const pollInterval = setInterval(async () => {
      try {
        // Poll proxy status (connected profile, uptime, actual ports)
        const status = await invoke<{
          status: string;
          activeProfileId: string | null;
          httpPort: number;
          socksPort: number;
          mixedPort: number;
          tunEnabled: boolean;
          uptime: number;
        }>('get_proxy_status');

        const active = status.status === 'connected';
        setIsConnected(active);
        setActiveProfileId(status.activeProfileId);
        setUptime(status.uptime);

        if (active) {
          // Poll real-time traffic statistics
          const stats = await invoke<{
            uploadBytes: number;
            downloadBytes: number;
            uploadSpeed: number;
            downloadSpeed: number;
            activeConnections: number;
          }>('get_traffic_stats');

          setUploadBytes(stats.uploadBytes);
          setDownloadBytes(stats.downloadBytes);
          setUploadSpeed(stats.uploadSpeed);
          setDownloadSpeed(stats.downloadSpeed);
          setActiveConnections(stats.activeConnections);

          // Update rolling canvas chart history (keep up to 30 points)
          setSpeedHistory((prev) => {
            const next = [...prev, { up: stats.uploadSpeed, down: stats.downloadSpeed }];
            return next.slice(-30);
          });
        } else {
          setUploadSpeed(0);
          setDownloadSpeed(0);
          setActiveConnections(0);
          // Insert idle speed point
          setSpeedHistory((prev) => {
            const next = [...prev, { up: 0, down: 0 }];
            return next.slice(-30);
          });
        }
      } catch (e) {
        console.error('Failed to poll traffic stats:', e);
      }
    }, 1000);

    return () => clearInterval(pollInterval);
  }, []);

  // Scroll terminal automatically if autoScrollLogs is active
  useEffect(() => {
    if (autoScrollLogs && consoleEndRef.current) {
      consoleEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [logs, autoScrollLogs]);

  // Load profile outbounds (nodes) when active/selected profile changes
  useEffect(() => {
    const loadOutbounds = async () => {
      if (!selectedProfileId) {
        setProfileOutbounds([]);
        setSelectedOutboundTag(null);
        return;
      }
      try {
        const outbounds = await invoke<any[]>('get_profile_outbounds', {
          profileId: selectedProfileId,
        });
        setProfileOutbounds(outbounds || []);

        const activeOutbound = await invoke<any>('get_profile_outbound', {
          profileId: selectedProfileId,
        });
        if (activeOutbound) {
          setSelectedOutboundTag(activeOutbound.tag);
        }
      } catch (e) {
        console.warn('Failed to load profile outbounds:', e);
        setProfileOutbounds([]);
      }
    };
    loadOutbounds();
  }, [selectedProfileId, profiles]);

  // Auto-select first profile if none selected but profiles exist
  useEffect(() => {
    if (profiles.length > 0 && !selectedProfileId) {
      setSelectedProfileId(profiles[0].id);
    }
  }, [profiles, selectedProfileId]);

  // Helper log functions
  const pushSystemLog = (text: string) => {
    setLogs((prev) => [...prev, { type: 'system', text: `[System] ${text}` }]);
  };

  // --- ACTION: CONNECT/DISCONNECT ---
  const handleToggleConnect = async () => {
    if (isConnected) {
      // Disconnect
      pushSystemLog('Stopping TunX tunnel proxy services...');
      try {
        const result = await invoke<string>('toggle_proxy', {
          start: false,
          profileId: '',
        });
        if (result === 'stopped') {
          setIsConnected(false);
          setActiveProfileId(null);
          pushSystemLog('Proxy tunnels stopped successfully. System proxy reverted.');
        }
      } catch (e) {
        pushSystemLog(`Error stopping proxy: ${e}`);
      }
    } else {
      // Connect
      if (!selectedProfileId) {
        pushSystemLog('Error: Please select or import a profile first.');
        setActiveTab('profiles');
        return;
      }

      // Check for port conflicts before starting
      try {
        const conflict = await invoke<number | null>('check_port_conflict', {
          ports: [httpPort, socksPort, mixedPort],
        });
        if (conflict) {
          pushSystemLog(`Error: Inbound port ${conflict} is already in use on your PC. Please close other proxy clients (like Clash, v2rayN) or change the Mixed/HTTP/SOCKS port in the "Settings" tab.`);
          setActiveTab('settings');
          return;
        }
      } catch (err) {
        console.warn('Port check skipped:', err);
      }

      const targetProfile = profiles.find((p) => p.id === selectedProfileId);
      pushSystemLog(`Booting TunX Core using profile "${targetProfile?.name || 'Default'}"...`);

      try {
        const result = await invoke<string>('toggle_proxy', {
          start: true,
          profileId: selectedProfileId,
        });
        if (result === 'started') {
          setIsConnected(true);
          setActiveProfileId(selectedProfileId);
          pushSystemLog(`sing-box tunnel established successfully on ports HTTP:${httpPort}, Mixed:${mixedPort}.`);
        }
      } catch (e) {
        pushSystemLog(`Startup error: ${e}`);
      }
    }
  };

  // --- ACTION: ELEVATION REQUEST ---
  const handleRequestElevation = async () => {
    pushSystemLog('Launching Administrator elevation sequence (UAC)...');
    try {
      await invoke('request_elevation');
    } catch (e) {
      pushSystemLog(`Elevation sequence aborted: ${e}`);
    }
  };

  // --- ACTION: REMOVE PROFILE ---
  const handleDeleteProfile = async (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await invoke('delete_profile', { profileId: id });
      await storeHelper.removeProfile(id);

      const remaining = profiles.filter((p) => p.id !== id);
      setProfiles(remaining);

      pushSystemLog('Profile configuration deleted.');
      if (selectedProfileId === id) {
        setSelectedProfileId(remaining.length > 0 ? remaining[0].id : null);
      }
    } catch (err) {
      pushSystemLog(`Failed to delete profile: ${err}`);
    }
  };

  // --- ACTION: SELECT OUTBOUND NODE ---
  const handleSelectOutbound = async (node: any) => {
    if (!selectedProfileId) return;
    try {
      pushSystemLog(`Switching primary route to node "${node.tag}"...`);
      await invoke('update_profile_config', {
        profileId: selectedProfileId,
        newOutbound: node,
      });
      setSelectedOutboundTag(node.tag);
      pushSystemLog(`Node switched to "${node.tag}". Configuration validated and patched.`);
    } catch (err) {
      pushSystemLog(`Failed to switch node: ${err}`);
    }
  };

  // --- ACTION: NATIVE FILE PICKER ---
  const handlePickFile = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'Configurations',
          extensions: ['json', 'yaml', 'yml', 'txt']
        }]
      });
      if (selected) {
        setImportFilePath(selected as string);
        if (!importName) {
          // Auto fill name
          const fileName = (selected as string).split(/[\\/]/).pop() || '';
          setImportName(fileName.replace(/\.[^/.]+$/, ''));
        }
      }
    } catch (err) {
      console.error(err);
    }
  };

  // --- ACTION: CLIPBOARD PASTER ---
  const handlePasteClipboard = async () => {
    try {
      const text = await readText();
      if (text) {
        setImportContent(text);
        pushSystemLog('Pasted configuration bytes from clipboard.');
      } else {
        setImportError('Clipboard is empty.');
      }
    } catch (err) {
      setImportError('Failed to read clipboard.');
    }
  };

  // --- ACTION: PROFILE IMPORT EXECUTE ---
  const handleImportProfile = async (e: React.FormEvent) => {
    e.preventDefault();
    setImportError(null);
    setImportSuccess(false);
    setIsImporting(true);

    if (!importName.trim()) {
      setImportError('Please enter a profile name.');
      setIsImporting(false);
      return;
    }

    try {
      let imported: Profile;

      if (importTab === 'url') {
        if (!importUrl.trim()) {
          setImportError('Please provide a valid subscription URL.');
          setIsImporting(false);
          return;
        }
        pushSystemLog(`Downloading subscription from provider: ${importUrl}`);
        imported = await invoke<Profile>('import_subscription', {
          url: importUrl,
          name: importName,
        });
      } else if (importTab === 'file') {
        if (!importFilePath.trim()) {
          setImportError('Please select a local config file.');
          setIsImporting(false);
          return;
        }
        pushSystemLog(`Loading local configuration file: ${importFilePath}`);
        imported = await invoke<Profile>('import_file', {
          filePath: importFilePath,
          name: importName,
        });
      } else {
        if (!importContent.trim()) {
          setImportError('Please paste configuration content.');
          setIsImporting(false);
          return;
        }
        pushSystemLog(`Parsing custom clipboard configuration payload...`);
        imported = await invoke<Profile>('import_from_clipboard', {
          content: importContent,
          name: importName,
        });
      }

      // Add to store helper
      await storeHelper.addProfile(imported);

      // Reload state
      const reloaded = await storeHelper.getProfiles();
      setProfiles(reloaded);
      setSelectedProfileId(imported.id);

      setImportSuccess(true);
      setImportName('');
      setImportUrl('');
      setImportContent('');
      setImportFilePath('');
      pushSystemLog(`Profile "${imported.name}" successfully validated and imported! Node Count: ${imported.nodeCount}`);

      // Clear success notification after 3s
      setTimeout(() => setImportSuccess(false), 3000);
    } catch (err) {
      console.error(err);
      setImportError(String(err));
      pushSystemLog(`Import failed: ${err}`);
    } finally {
      setIsImporting(false);
    }
  };

  // --- ACTION: SAVE SETTINGS ---
  const handleSaveSettings = async (updates: Partial<Settings>) => {
    const nextSettings = { ...settings, ...updates };
    setSettings(nextSettings);
    await storeHelper.saveSettings(updates);

    if (updates.autostart !== undefined) {
      try {
        await invoke('set_autostart', { enabled: updates.autostart });
        pushSystemLog(`System Autostart ${updates.autostart ? 'enabled' : 'disabled'}.`);
      } catch (e) {
        pushSystemLog(`Failed to configure autostart: ${e}`);
      }
    }
  };

  // --- ACTION: OPEN CONFIGURATION EDITOR ---
  const handleOpenEditor = (node: any) => {
    setEditSaveError(null);
    setEditTag(node.tag || '');
    setEditAddress(node.server || '');
    setEditPort(node.server_port || 443);
    setEditProtocol(node.type || 'vless');
    setEditUuid(node.uuid || node.id || '');
    setEditFlow(node.flow || '');
    setEditPassword(node.password || '');
    setEditMethod(node.method || '');
    
    const transportType = node.transport?.type || node.network || 'tcp';
    setEditNetwork(transportType === 'ws' || transportType === 'grpc' ? transportType : 'tcp');
    setEditHeaderType(node.transport?.headers?.type || '');
    setEditPath(node.transport?.path || '');
    setEditHost(node.transport?.headers?.Host || node.transport?.host || '');
    
    setEditTlsEnabled(!!node.tls?.enabled);
    setEditAllowInsecure(!!node.tls?.insecure);
    setEditServerName(node.tls?.server_name || '');
    setEditAlpn(node.tls?.alpn?.join(', ') || '');
    setEditRealityEnabled(!!node.tls?.reality?.enabled);
    setEditFingerprint(node.tls?.utls?.fingerprint || '');
    setEditPublicKey(node.tls?.reality?.public_key || '');
    setEditShortId(node.tls?.reality?.short_id || '');

    setIsEditingConfig(true);
  };

  // --- ACTION: COMPILE AND SAVE CONFIGURATION ---
  const handleSaveEditor = async () => {
    if (!selectedProfileId) return;
    setEditSaveError(null);
    setIsSavingConfig(true);

    if (!editTag.trim()) {
      setEditSaveError('Please provide a Tag name.');
      setIsSavingConfig(false);
      return;
    }
    if (!editAddress.trim()) {
      setEditSaveError('Please provide a Server Address.');
      setIsSavingConfig(false);
      return;
    }

    try {
      const outbound: any = {
        type: editProtocol,
        tag: editTag.trim(),
        server: editAddress.trim(),
        server_port: Number(editPort) || 443,
      };

      // Protocol-specific mappings
      if (editProtocol === 'vless' || editProtocol === 'vmess') {
        if (!editUuid.trim()) {
          setEditSaveError('A valid UUID is required for VLESS/VMess.');
          setIsSavingConfig(false);
          return;
        }
        outbound.uuid = editUuid.trim();
        if (editFlow && editProtocol === 'vless') {
          outbound.flow = editFlow;
        }
      } else if (editProtocol === 'trojan') {
        if (!editPassword.trim()) {
          setEditSaveError('Password is required for Trojan.');
          setIsSavingConfig(false);
          return;
        }
        outbound.password = editPassword.trim();
      } else if (editProtocol === 'shadowsocks') {
        if (!editPassword.trim()) {
          setEditSaveError('Password is required for Shadowsocks.');
          setIsSavingConfig(false);
          return;
        }
        outbound.method = editMethod || 'aes-256-gcm';
        outbound.password = editPassword.trim();
      }

      // TLS Security settings
      if (editTlsEnabled) {
        const tls: any = {
          enabled: true,
        };
        if (editServerName.trim()) {
          tls.server_name = editServerName.trim();
        }
        if (editAllowInsecure) {
          tls.insecure = true;
        }
        if (editAlpn.trim()) {
          tls.alpn = editAlpn.split(',').map((s) => s.trim()).filter(Boolean);
        }
        if (editFingerprint) {
          tls.utls = {
            enabled: true,
            fingerprint: editFingerprint,
          };
        }
        if (editRealityEnabled) {
          if (!editPublicKey.trim()) {
            setEditSaveError('Reality Public Key is required when Reality is enabled.');
            setIsSavingConfig(false);
            return;
          }
          tls.reality = {
            enabled: true,
            public_key: editPublicKey.trim(),
            short_id: editShortId.trim() || undefined,
          };
        }
        outbound.tls = tls;
      }

      // Transport settings
      if (editNetwork && editNetwork !== 'tcp') {
        const transport: any = {
          type: editNetwork,
        };
        if (editPath.trim()) {
          transport.path = editPath.trim();
        }
        if (editHost.trim()) {
          transport.headers = {
            Host: editHost.trim(),
          };
        }
        outbound.transport = transport;
      } else if (editNetwork === 'tcp') {
        if (editHeaderType || editPath || editHost) {
          const transport: any = {
            type: 'tcp',
          };
          if (editHeaderType) {
            transport.headers = {
              type: editHeaderType,
            };
          }
          if (editPath.trim()) {
            transport.path = editPath.trim();
          }
          if (editHost.trim()) {
            transport.headers = {
              ...transport.headers,
              Host: editHost.trim(),
            };
          }
          outbound.transport = transport;
        }
      }

      // Call Rust backend validator & updater
      pushSystemLog(`Saving updated node config "${outbound.tag}"...`);
      await invoke('update_profile_config', {
        profileId: selectedProfileId,
        newOutbound: outbound,
      });

      // Reload profile outbounds
      const outbounds = await invoke<any[]>('get_profile_outbounds', {
        profileId: selectedProfileId,
      });
      setProfileOutbounds(outbounds || []);
      
      const activeOutbound = await invoke<any>('get_profile_outbound', {
        profileId: selectedProfileId,
      });
      if (activeOutbound) {
        setSelectedOutboundTag(activeOutbound.tag);
      }

      pushSystemLog(`Outbound node "${outbound.tag}" successfully updated and validated.`);
      setIsEditingConfig(false);
    } catch (err) {
      console.error(err);
      setEditSaveError(String(err));
      pushSystemLog(`Failed to update outbound node configuration: ${err}`);
    } finally {
      setIsSavingConfig(false);
    }
  };

  return (
    <div className="app-container">
      {/* LEFT SIDEBAR NAVIGATION */}
      <aside className="sidebar">
        <div className="brand-section">
          <div className="brand-logo-container">
            <Zap size={22} color="#0B0C10" strokeWidth={3} />
            <div className="brand-logo-glow"></div>
          </div>
          <span className="brand-title">TunX</span>
        </div>

        <nav className="nav-list">
          <div
            className={`nav-item ${activeTab === 'dashboard' ? 'active' : ''}`}
            onClick={() => setActiveTab('dashboard')}
          >
            <Activity />
            <span>Dashboard</span>
          </div>
          <div
            className={`nav-item ${activeTab === 'profiles' ? 'active' : ''}`}
            onClick={() => setActiveTab('profiles')}
          >
            <Layers />
            <span>Profiles</span>
          </div>
          <div
            className={`nav-item ${activeTab === 'logs' ? 'active' : ''}`}
            onClick={() => setActiveTab('logs')}
          >
            <Terminal />
            <span>Live Logs</span>
          </div>
          <div
            className={`nav-item ${activeTab === 'settings' ? 'active' : ''}`}
            onClick={() => setActiveTab('settings')}
          >
            <SettingsIcon />
            <span>Settings</span>
          </div>
        </nav>

        <div className="sidebar-footer">
          <div className="status-badge-container">
            <div className={`status-dot ${isConnected ? 'connected' : 'idle'}`}></div>
            <span className="status-text" style={{ color: isConnected ? 'var(--status-connected)' : 'var(--status-idle)' }}>
              {isConnected ? 'connected' : 'idle'}
            </span>
          </div>
        </div>
      </aside>

      {/* RIGHT MAIN WORKSPACE */}
      <main className="main-content">
        {/* --- VIEW TABS ROUTER --- */}

        {/* 1. VIEW: DASHBOARD OVERVIEW */}
        {activeTab === 'dashboard' && (
          <div className="view-container">
            <header className="view-header">
              <div>
                <h1 className="view-title">Dashboard</h1>
                <p className="view-subtitle">Real-time status monitor and proxy controls</p>
              </div>
              <div className="flex-row-between gap-12">
                {isElevated ? (
                  <div className="badge flex-row-between gap-12" style={{ background: 'rgba(16, 185, 129, 0.08)', border: '1px solid rgba(16, 185, 129, 0.2)', padding: '6px 12px', borderRadius: '20px', fontSize: '12px', color: 'var(--status-connected)', fontWeight: 500 }}>
                    <Shield size={14} />
                    <span>Elevated Mode (TUN Active)</span>
                  </div>
                ) : (
                  <button
                    className="btn btn-secondary flex-row-between gap-12"
                    style={{ padding: '6px 12px', borderRadius: '20px', fontSize: '12px' }}
                    onClick={handleRequestElevation}
                  >
                    <ShieldAlert size={14} className="text-danger" />
                    <span>Run as Admin (UAC)</span>
                  </button>
                )}
              </div>
            </header>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
              <div className="grid-3">
                {/* Connection switch */}
                <div className="glass-panel connect-panel" style={{ gridColumn: 'span 1' }}>
                  <div className="power-button-container">
                    <div className={`power-button-outer ${isConnected ? 'connected' : ''}`}></div>
                    <button
                      className={`power-button ${isConnected ? 'connected' : ''}`}
                      onClick={handleToggleConnect}
                    >
                      <Power />
                    </button>
                  </div>
                  <h3 className="connect-status-label">
                    {isConnected ? 'Connected' : 'Disconnected'}
                  </h3>
                  <p className="connect-status-sub">
                    {isConnected
                      ? `Active Profile: ${profiles.find((p) => p.id === activeProfileId)?.name || 'Default'}`
                      : 'Toggle power button to start proxy'}
                  </p>
                </div>

                {/* Speed Stats Chart */}
                <div className="glass-panel chart-panel" style={{ gridColumn: 'span 2' }}>
                  <div className="chart-header">
                    <h3 className="chart-title">Bandwidth Usage</h3>
                    <div className="chart-legends">
                      <div className="legend-item">
                        <div className="legend-dot download"></div>
                        <span>Down: {formatSpeed(downloadSpeed)}</span>
                      </div>
                      <div className="legend-item">
                        <div className="legend-dot upload"></div>
                        <span>Up: {formatSpeed(uploadSpeed)}</span>
                      </div>
                    </div>
                  </div>
                  <div className="chart-body">
                    <TrafficChart history={speedHistory} />
                  </div>
                </div>
              </div>

              {/* Status Metrics Cards Grid */}
              <div className="grid-4">
                <div className="glass-panel metric-card">
                  <div className="metric-icon-box">
                    <DownloadCloud size={20} />
                  </div>
                  <div className="metric-info">
                    <span className="metric-label">Total Down</span>
                    <span className="metric-value">{formatBytes(downloadBytes)}</span>
                  </div>
                </div>

                <div className="glass-panel metric-card">
                  <div className="metric-icon-box">
                    <UploadCloud size={20} />
                  </div>
                  <div className="metric-info">
                    <span className="metric-label">Total Up</span>
                    <span className="metric-value">{formatBytes(uploadBytes)}</span>
                  </div>
                </div>

                <div className="glass-panel metric-card">
                  <div className="metric-icon-box">
                    <Clock size={20} />
                  </div>
                  <div className="metric-info">
                    <span className="metric-label">Uptime</span>
                    <span className="metric-value">{formatUptime(uptime)}</span>
                  </div>
                </div>

                <div className="glass-panel metric-card">
                  <div className="metric-icon-box">
                    <Server size={20} />
                  </div>
                  <div className="metric-info">
                    <span className="metric-label">Conns</span>
                    <span className="metric-value">{activeConnections} Active</span>
                  </div>
                </div>
              </div>

              {/* System Active Ports & Outbound Selector */}
              <div className="glass-panel">
                <div className="flex-row-between" style={{ marginBottom: '16px' }}>
                  <h3 style={{ fontSize: '16px', fontWeight: 600 }}>Active Profiles Outbound</h3>
                  <div style={{ display: 'flex', gap: '20px', fontSize: '13px', color: 'var(--text-secondary)' }}>
                    <span>HTTP Port: <strong style={{ color: 'var(--text-primary)' }}>{httpPort}</strong></span>
                    <span>SOCKS Port: <strong style={{ color: 'var(--text-primary)' }}>{socksPort}</strong></span>
                    <span>Mixed Port: <strong style={{ color: 'var(--text-primary)' }}>{mixedPort}</strong></span>
                  </div>
                </div>
                {selectedProfileId ? (
                  <>
                    <p style={{ fontSize: '13px', color: 'var(--text-secondary)' }}>
                      Select which proxy outbound node to route traffic through:
                    </p>
                    {profileOutbounds.length > 0 ? (
                      <div className="node-grid">
                        {profileOutbounds.map((node, i) => (
                          <div
                            key={i}
                            className={`node-card ${selectedOutboundTag === node.tag ? 'active' : ''}`}
                            onClick={() => handleSelectOutbound(node)}
                          >
                            <span className="node-name" title={node.tag}>{node.tag}</span>
                            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                              <span className="node-type-badge">{node.type}</span>
                              <button
                                className="btn-icon-only"
                                style={{ width: '22px', height: '22px', border: 'none', background: 'transparent', color: 'var(--text-secondary)' }}
                                onClick={(e) => {
                                  e.stopPropagation();
                                  handleOpenEditor(node);
                                }}
                                title="Edit Outbound Node Settings"
                              >
                                <SettingsIcon size={12} />
                              </button>
                            </div>
                          </div>
                        ))}
                      </div>
                    ) : (
                      <p style={{ marginTop: '16px', fontSize: '13px', color: 'var(--text-muted)' }}>
                        No customizable outbounds found. All traffic goes directly or is handled by profile rules.
                      </p>
                    )}
                  </>
                ) : (
                  <p style={{ fontSize: '13px', color: 'var(--text-muted)' }}>
                    No profiles loaded. Go to the "Profiles" tab to add one.
                  </p>
                )}
              </div>
            </div>
          </div>
        )}

        {/* 2. VIEW: PROFILE MANAGER */}
        {activeTab === 'profiles' && (
          <div className="view-container">
            <header className="view-header">
              <div>
                <h1 className="view-title">Profiles</h1>
                <p className="view-subtitle">Import and manage proxy subscription profiles</p>
              </div>
            </header>

            <div className="grid-2">
              {/* Profile Selection list */}
              <div className="glass-panel" style={{ height: 'calc(100vh - 180px)', display: 'flex', flexDirection: 'column' }}>
                <h3 style={{ fontSize: '16px', fontWeight: 600, marginBottom: '16px' }}>Your Configs</h3>
                <div style={{ flexGrow: 1, overflowY: 'auto', paddingRight: '4px' }}>
                  {profiles.length > 0 ? (
                    profiles.map((p) => (
                      <div
                        key={p.id}
                        className={`profile-item ${selectedProfileId === p.id ? 'active' : ''}`}
                        onClick={() => setSelectedProfileId(p.id)}
                      >
                        <div className="profile-item-left">
                          <div className="profile-icon">
                            <Globe size={18} />
                          </div>
                          <div className="profile-details">
                            <span className="profile-name">{p.name}</span>
                            <div className="profile-meta">
                              <span style={{ textTransform: 'capitalize' }}>{p.type}</span>
                              <span>•</span>
                              <span>{p.nodeCount} nodes</span>
                            </div>
                          </div>
                        </div>
                        <div className="profile-actions">
                          {activeProfileId === p.id && isConnected && (
                            <span style={{ background: 'rgba(16, 185, 129, 0.1)', color: 'var(--status-connected)', border: '1px solid rgba(16, 185, 129, 0.2)', padding: '3px 8px', borderRadius: '12px', fontSize: '10px', fontWeight: 600, marginRight: '4px', display: 'flex', alignItems: 'center' }}>
                              ACTIVE
                            </span>
                          )}
                          <button
                            className="btn-icon-only danger"
                            onClick={(e) => handleDeleteProfile(p.id, e)}
                            title="Delete configuration"
                          >
                            <Trash2 size={15} />
                          </button>
                        </div>
                      </div>
                    ))
                  ) : (
                    <div style={{ textAlign: 'center', color: 'var(--text-muted)', marginTop: '60px' }}>
                      <Globe size={40} style={{ strokeWidth: 1, marginBottom: '12px' }} />
                      <p style={{ fontSize: '14px' }}>No profiles found</p>
                      <p style={{ fontSize: '12px', marginTop: '4px' }}>Import a config using the panel on the right.</p>
                    </div>
                  )}
                </div>
              </div>

              {/* Profile Import Hub */}
              <div className="glass-panel" style={{ height: 'calc(100vh - 180px)', display: 'flex', flexDirection: 'column' }}>
                <h3 style={{ fontSize: '16px', fontWeight: 600, marginBottom: '16px' }}>Import Profile</h3>

                <div className="tab-row">
                  <button
                    className={`tab-btn ${importTab === 'url' ? 'active' : ''}`}
                    onClick={() => setImportTab('url')}
                  >
                    Subscription URL
                  </button>
                  <button
                    className={`tab-btn ${importTab === 'file' ? 'active' : ''}`}
                    onClick={() => setImportTab('file')}
                  >
                    Local File
                  </button>
                  <button
                    className={`tab-btn ${importTab === 'clipboard' ? 'active' : ''}`}
                    onClick={() => setImportTab('clipboard')}
                  >
                    Clipboard
                  </button>
                </div>

                <form onSubmit={handleImportProfile} style={{ display: 'flex', flexDirection: 'column', gap: '16px', flexGrow: 1 }}>
                  <div className="form-group">
                    <label className="form-label">Profile Name</label>
                    <input
                      type="text"
                      className="text-input"
                      placeholder="e.g. Premium Proxy Server"
                      value={importName}
                      onChange={(e) => setImportName(e.target.value)}
                    />
                  </div>

                  {importTab === 'url' && (
                    <div className="form-group">
                      <label className="form-label">Subscription Link</label>
                      <input
                        type="url"
                        className="text-input"
                        placeholder="https://provider.com/clash.yaml"
                        value={importUrl}
                        onChange={(e) => setImportUrl(e.target.value)}
                      />
                    </div>
                  )}

                  {importTab === 'file' && (
                    <div className="form-group">
                      <label className="form-label">Configuration File</label>
                      <div style={{ display: 'flex', gap: '10px' }}>
                        <input
                          type="text"
                          className="text-input"
                          placeholder="Select JSON, YAML, or Base64 file..."
                          value={importFilePath}
                          readOnly
                        />
                        <button
                          type="button"
                          className="btn btn-secondary"
                          onClick={handlePickFile}
                        >
                          <FolderOpen size={16} />
                          Browse
                        </button>
                      </div>
                    </div>
                  )}

                  {importTab === 'clipboard' && (
                    <div className="form-group" style={{ flexGrow: 1, display: 'flex', flexDirection: 'column' }}>
                      <div className="flex-row-between">
                        <label className="form-label">Raw Config Content (Clash/yaml or base64)</label>
                        <button
                          type="button"
                          className="btn btn-secondary"
                          style={{ padding: '4px 8px', fontSize: '11px', borderRadius: '4px' }}
                          onClick={handlePasteClipboard}
                        >
                          <Clipboard size={12} />
                          Paste Clipboard
                        </button>
                      </div>
                      <textarea
                        className="text-input"
                        style={{ height: '140px', fontFamily: 'var(--font-mono)', fontSize: '12px', resize: 'none', flexGrow: 1, marginTop: '8px' }}
                        placeholder="Paste subscription bytes, clash yaml YAML, or sing-box JSON configuration content here..."
                        value={importContent}
                        onChange={(e) => setImportContent(e.target.value)}
                      />
                    </div>
                  )}

                  {importError && (
                    <div style={{ background: 'rgba(239, 68, 68, 0.08)', border: '1px solid rgba(239, 68, 68, 0.2)', padding: '10px', borderRadius: '8px', fontSize: '12px', color: 'var(--status-error)', display: 'flex', gap: '8px', alignItems: 'center' }}>
                      <ShieldAlert size={14} style={{ flexShrink: 0 }} />
                      <span style={{ wordBreak: 'break-word' }}>{importError}</span>
                    </div>
                  )}

                  {importSuccess && (
                    <div style={{ background: 'rgba(16, 185, 129, 0.08)', border: '1px solid rgba(16, 185, 129, 0.2)', padding: '10px', borderRadius: '8px', fontSize: '12px', color: 'var(--status-connected)', display: 'flex', gap: '8px', alignItems: 'center' }}>
                      <Check size={14} style={{ flexShrink: 0 }} />
                      <span>Profile imported and successfully validated by sing-box!</span>
                    </div>
                  )}

                  <button
                    type="submit"
                    className="btn btn-primary"
                    disabled={isImporting}
                    style={{ marginTop: 'auto', width: '100%', height: '44px' }}
                  >
                    {isImporting ? (
                      <>
                        <RefreshCw size={16} className="spin" />
                        Validating Config...
                      </>
                    ) : (
                      <>
                        <Plus size={18} />
                        Import & Validate Configuration
                      </>
                    )}
                  </button>
                </form>
              </div>
            </div>
          </div>
        )}

        {/* 3. VIEW: LIVE LOGS CONSOLE */}
        {activeTab === 'logs' && (
          <div className="view-container">
            <header className="view-header">
              <div>
                <h1 className="view-title">Live Logs</h1>
                <p className="view-subtitle">Real-time sing-box system process console logs</p>
              </div>
            </header>

            <div className="glass-panel console-panel">
              <div className="console-header">
                <div className="console-title-box">
                  <Terminal size={18} className="text-success" />
                  <span style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-secondary)' }}>sing-box output terminal</span>
                </div>
                <div style={{ display: 'flex', gap: '12px' }}>
                  <button
                    className="btn btn-secondary"
                    style={{ padding: '6px 12px', fontSize: '12px' }}
                    onClick={() => {
                      const text = logs.map((l) => l.text).join('\n');
                      navigator.clipboard.writeText(text);
                      pushSystemLog('Copied all terminal logs to clipboard.');
                    }}
                  >
                    <Clipboard size={14} />
                    Copy Logs
                  </button>
                  <button
                    className="btn btn-secondary"
                    style={{ padding: '6px 12px', fontSize: '12px' }}
                    onClick={() => setLogs([])}
                  >
                    Clear Console
                  </button>
                  <label className="switch-toggle" style={{ transform: 'scale(0.85)', margin: 'auto' }} title="Autoscroll logs">
                    <input
                      type="checkbox"
                      checked={autoScrollLogs}
                      onChange={(e) => setAutoScrollLogs(e.target.checked)}
                    />
                    <span className="switch-slider"></span>
                  </label>
                  <span style={{ fontSize: '12px', color: 'var(--text-secondary)', margin: 'auto 0 auto 4px' }}>Autoscroll</span>
                </div>
              </div>

              <div className="console-terminal">
                {logs.length > 0 ? (
                  logs.map((log, index) => (
                    <div key={index} className={`console-line ${log.type}`}>
                      {log.text}
                    </div>
                  ))
                ) : (
                  <div style={{ textAlign: 'center', color: 'var(--text-muted)', height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center', flexDirection: 'column' }}>
                    <Terminal size={32} style={{ strokeWidth: 1, marginBottom: '8px' }} />
                    <span>Output terminal waiting for live process logs...</span>
                  </div>
                )}
                <div ref={consoleEndRef} />
              </div>
            </div>
          </div>
        )}

        {/* 4. VIEW: SETTINGS */}
        {activeTab === 'settings' && (
          <div className="view-container">
            <header className="view-header">
              <div>
                <h1 className="view-title">Settings</h1>
                <p className="view-subtitle">Configure ports, modes, autostart and engine setups</p>
              </div>
            </header>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
              <div className="grid-2">
                {/* General Settings */}
                <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                  <h3 style={{ fontSize: '16px', fontWeight: 600, marginBottom: '6px' }}>Proxy Modes & Behaviors</h3>

                  <div className="switch-container">
                    <div className="switch-details">
                      <span className="switch-title">Virtual TUN Interface</span>
                      <span className="switch-desc">Route all PC traffic natively. Requires Administrator elevation.</span>
                    </div>
                    <label className="switch-toggle">
                      <input
                        type="checkbox"
                        checked={settings.proxyMode === 'tun'}
                        disabled={!isElevated && settings.proxyMode !== 'tun'}
                        onChange={(e) => {
                          const checked = e.target.checked;
                          handleSaveSettings({ proxyMode: checked ? 'tun' : 'system' });
                        }}
                      />
                      <span className="switch-slider"></span>
                    </label>
                  </div>

                  <div className="switch-container">
                    <div className="switch-details">
                      <span className="switch-title">Close to Tray</span>
                      <span className="switch-desc">Minimize window to system tray menu upon hitting close.</span>
                    </div>
                    <label className="switch-toggle">
                      <input
                        type="checkbox"
                        checked={settings.closeToTray}
                        onChange={(e) => handleSaveSettings({ closeToTray: e.target.checked })}
                      />
                      <span className="switch-slider"></span>
                    </label>
                  </div>

                  <div className="switch-container">
                    <div className="switch-details">
                      <span className="switch-title">Autostart with Windows</span>
                      <span className="switch-desc">Launch and register TunX task minimized in user login.</span>
                    </div>
                    <label className="switch-toggle">
                      <input
                        type="checkbox"
                        checked={settings.autostart}
                        onChange={(e) => handleSaveSettings({ autostart: e.target.checked })}
                      />
                      <span className="switch-slider"></span>
                    </label>
                  </div>
                </div>

                {/* Ports Configurations */}
                <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                  <h3 style={{ fontSize: '16px', fontWeight: 600, marginBottom: '6px' }}>Inbound Ports Settings</h3>

                  <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px' }}>
                    <div className="form-group">
                      <div className="flex-row-between">
                        <label className="form-label">HTTP Proxy Port</label>
                        {conflictingPorts.includes(httpPort) ? (
                          <span className="text-danger" style={{ fontSize: '11px', fontWeight: 600 }}>● Conflict!</span>
                        ) : (
                          <span className="text-success" style={{ fontSize: '11px', fontWeight: 500 }}>● Available</span>
                        )}
                      </div>
                      <input
                        type="number"
                        className="text-input"
                        style={{ borderColor: conflictingPorts.includes(httpPort) ? 'var(--status-error)' : undefined }}
                        value={httpPort}
                        onChange={(e) => {
                          const val = parseInt(e.target.value) || 0;
                          setHttpPort(val);
                          handleSaveSettings({ httpPort: val });
                        }}
                      />
                    </div>
                    <div className="form-group">
                      <div className="flex-row-between">
                        <label className="form-label">SOCKS5 Port</label>
                        {conflictingPorts.includes(socksPort) ? (
                          <span className="text-danger" style={{ fontSize: '11px', fontWeight: 600 }}>● Conflict!</span>
                        ) : (
                          <span className="text-success" style={{ fontSize: '11px', fontWeight: 500 }}>● Available</span>
                        )}
                      </div>
                      <input
                        type="number"
                        className="text-input"
                        style={{ borderColor: conflictingPorts.includes(socksPort) ? 'var(--status-error)' : undefined }}
                        value={socksPort}
                        onChange={(e) => {
                          const val = parseInt(e.target.value) || 0;
                          setSocksPort(val);
                          handleSaveSettings({ socksPort: val });
                        }}
                      />
                    </div>
                  </div>

                  <div className="form-group">
                    <div className="flex-row-between">
                      <label className="form-label">Mixed Inbound Port (Recommended)</label>
                      {conflictingPorts.includes(mixedPort) ? (
                        <span className="text-danger" style={{ fontSize: '11px', fontWeight: 600 }}>● Conflict!</span>
                      ) : (
                        <span className="text-success" style={{ fontSize: '11px', fontWeight: 500 }}>● Available</span>
                      )}
                    </div>
                    <input
                      type="number"
                      className="text-input"
                      style={{ borderColor: conflictingPorts.includes(mixedPort) ? 'var(--status-error)' : undefined }}
                      value={mixedPort}
                      onChange={(e) => {
                        const val = parseInt(e.target.value) || 0;
                        setMixedPort(val);
                        handleSaveSettings({ mixedPort: val });
                      }}
                    />
                  </div>

                  <p style={{ fontSize: '12px', color: 'var(--text-muted)', lineHeight: 1.5 }}>
                    Note: Changing inbound ports will automatically patch active configurations. If the proxy is running, you'll need to restart the connection to bind new ports.
                  </p>
                </div>
              </div>

              {/* Bypass, DNS Routing & Hotspot Sharing Panel */}
              <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                  <Shield size={20} style={{ color: 'var(--accent-cyan)' }} />
                  <h3 style={{ fontSize: '16px', fontWeight: 600 }}>Advanced SNI Tunneling & DNS Routing</h3>
                </div>

                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '24px' }}>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                    <div className="form-group">
                      <label className="form-label">SNI Bypass Hostname (ISP Allowed)</label>
                      <input
                        type="text"
                        className="text-input"
                        value={settings.sniHost}
                        onChange={(e) => handleSaveSettings({ sniHost: e.target.value })}
                        placeholder="aka.ms"
                      />
                      <span style={{ fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px', display: 'block', lineHeight: '1.4' }}>
                        Allows spoofing TLS SNI host (e.g., aka.ms, zoom.us) to bypass ISP throttling or leverage zero-rated packages.
                      </span>
                    </div>

                    <div className="form-group">
                      <label className="form-label">Direct DNS Server (Router IP / Gateway)</label>
                      <input
                        type="text"
                        className="text-input"
                        value={settings.dnsAddress}
                        onChange={(e) => handleSaveSettings({ dnsAddress: e.target.value })}
                        placeholder="Auto (system DNS)"
                      />
                      <span style={{ fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px', display: 'block', lineHeight: '1.4' }}>
                        DNS queries resolve directly through your WiFi router's ISP DNS (Gateway IP) to blend DNS traffic cleanly. Leave empty to use system DNS.
                      </span>
                    </div>
                  </div>

                  <div style={{ display: 'flex', flexDirection: 'column', gap: '16px', justifyContent: 'center' }}>
                    <div className="switch-container" style={{ padding: '16px', background: 'rgba(255, 255, 255, 0.01)', border: '1px solid var(--border-glass)', borderRadius: '12px' }}>
                      <div className="switch-details">
                        <span className="switch-title" style={{ fontSize: '14px', fontWeight: 600 }}>LAN Hotspot Proxy Sharing</span>
                        <span className="switch-desc" style={{ fontSize: '12px', color: 'var(--text-secondary)', marginTop: '2px' }}>
                          Bind proxy service to <code>0.0.0.0</code> to allow other devices (mobiles, TVs, consoles) on your WiFi network to connect.
                        </span>
                      </div>
                      <label className="switch-toggle" style={{ marginLeft: '12px' }}>
                        <input
                          type="checkbox"
                          checked={settings.wifiSharing}
                          onChange={(e) => handleSaveSettings({ wifiSharing: e.target.checked })}
                        />
                        <span className="switch-slider"></span>
                      </label>
                    </div>

                    <p style={{ fontSize: '12px', color: 'var(--text-muted)', lineHeight: '1.5' }}>
                      Tip: Active tunnels will require a <strong>Reconnect</strong> to enforce new SNI detours, WiFi sharing address, or custom ISP DNS configurations.
                    </p>
                  </div>
                </div>
              </div>

              {/* Version & Credits Panel */}
              <div className="glass-panel" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '24px 30px' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '20px' }}>
                  <div className="metric-icon-box" style={{ width: '56px', height: '56px', borderRadius: '14px', background: 'rgba(168, 85, 247, 0.05)', color: 'var(--accent-purple)', borderColor: 'rgba(168, 85, 247, 0.15)' }}>
                    <Cpu size={24} />
                  </div>
                  <div>
                    <h4 style={{ fontSize: '16px', fontWeight: 600 }}>TunX Engine Specifications</h4>
                    <p style={{ fontSize: '13px', color: 'var(--text-secondary)', marginTop: '2px' }}>
                      Sing-box Version: <strong style={{ color: 'var(--text-primary)' }}>{singboxVersion.split('\n')[0] || singboxVersion}</strong>
                    </p>
                  </div>
                </div>
                <div style={{ textAlign: 'right', fontSize: '13px', color: 'var(--text-secondary)' }}>
                  <span>App Version: <strong style={{ color: 'var(--text-primary)' }}>v{appVersion}</strong></span>
                  <br />
                  <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Created by DeepMind Antigravity</span>
                </div>
              </div>
            </div>
          </div>
        )}
      </main>
      
      {/* ── HIGH FIDELITY NODE CONFIGURATION EDITOR MODAL ── */}
      {isEditingConfig && (
        <div className="editor-overlay">
          <div className="editor-modal">
            <header className="editor-header">
              <span className="editor-title">Edit Outbound Node Settings</span>
              <button
                className="btn-icon-only"
                onClick={() => setIsEditingConfig(false)}
                style={{ border: 'none', background: 'transparent' }}
              >
                <Trash2 size={16} style={{ transform: 'rotate(45deg)', color: 'var(--text-secondary)' }} />
              </button>
            </header>

            <div className="editor-body">
              <div className="editor-split-columns">
                {/* LEFT COLUMN: Common & Protocol Specific */}
                <div className="editor-column">
                  <div className="editor-subcard">
                    <span className="editor-subcard-title">Common</span>
                    <div className="form-group">
                      <label className="form-label">Name*</label>
                      <input
                        type="text"
                        className="text-input"
                        placeholder="zoom-One-Piece"
                        value={editTag}
                        onChange={(e) => setEditTag(e.target.value)}
                      />
                    </div>
                    <div className="form-group">
                      <label className="form-label">Address*</label>
                      <input
                        type="text"
                        className="text-input"
                        placeholder="azureedge.duckdns.org"
                        value={editAddress}
                        onChange={(e) => setEditAddress(e.target.value)}
                      />
                    </div>
                    <div className="form-group">
                      <label className="form-label">Port*</label>
                      <input
                        type="number"
                        className="text-input"
                        placeholder="443"
                        value={editPort}
                        onChange={(e) => setEditPort(Number(e.target.value) || 0)}
                      />
                    </div>
                  </div>

                  <div className="editor-subcard">
                    <span className="editor-subcard-title">Protocol Specification</span>
                    <div className="form-group">
                      <label className="form-label">Protocol Type</label>
                      <select
                        className="select-input"
                        value={editProtocol}
                        onChange={(e) => setEditProtocol(e.target.value)}
                      >
                        <option value="vless">VLESS</option>
                        <option value="vmess">VMess</option>
                        <option value="trojan">Trojan</option>
                        <option value="shadowsocks">Shadowsocks</option>
                      </select>
                    </div>

                    {(editProtocol === 'vless' || editProtocol === 'vmess') && (
                      <>
                        <div className="form-group">
                          <label className="form-label">UUID*</label>
                          <input
                            type="text"
                            className="text-input"
                            placeholder="a20d4836-4634-471b-b6c3-2f48acb0e9a5"
                            value={editUuid}
                            onChange={(e) => setEditUuid(e.target.value)}
                          />
                        </div>
                        {editProtocol === 'vless' && (
                          <div className="form-group">
                            <label className="form-label">Flow</label>
                            <select
                              className="select-input"
                              value={editFlow}
                              onChange={(e) => setEditFlow(e.target.value)}
                            >
                              <option value="">None (Default)</option>
                              <option value="xtls-rprx-vision">xtls-rprx-vision</option>
                            </select>
                          </div>
                        )}
                      </>
                    )}

                    {(editProtocol === 'trojan' || editProtocol === 'shadowsocks') && (
                      <div className="form-group">
                        <label className="form-label">Password*</label>
                        <input
                          type="password"
                          className="text-input"
                          placeholder="Your password..."
                          value={editPassword}
                          onChange={(e) => setEditPassword(e.target.value)}
                        />
                      </div>
                    )}

                    {editProtocol === 'shadowsocks' && (
                      <div className="form-group">
                        <label className="form-label">Encryption Method*</label>
                        <select
                          className="select-input"
                          value={editMethod}
                          onChange={(e) => setEditMethod(e.target.value)}
                        >
                          <option value="aes-256-gcm">aes-256-gcm</option>
                          <option value="aes-128-gcm">aes-128-gcm</option>
                          <option value="chacha20-ietf-poly1305">chacha20-ietf-poly1305</option>
                          <option value="2022-blake3-aes-256-gcm">2022-blake3-aes-256-gcm</option>
                        </select>
                      </div>
                    )}
                  </div>
                </div>

                {/* RIGHT COLUMN: Settings, TLS & Camouflage */}
                <div className="editor-column">
                  <div className="editor-subcard">
                    <span className="editor-subcard-title">Settings</span>
                    <div className="editor-form-grid-2">
                      <div className="form-group">
                        <label className="form-label">Network*</label>
                        <select
                          className="select-input"
                          value={editNetwork}
                          onChange={(e) => setEditNetwork(e.target.value as any)}
                        >
                          <option value="tcp">tcp</option>
                          <option value="ws">ws (WebSocket)</option>
                          <option value="grpc">gRPC</option>
                        </select>
                      </div>

                      <div className="form-group">
                        <label className="form-label">Security*</label>
                        <select
                          className="select-input"
                          value={editTlsEnabled ? (editRealityEnabled ? 'reality' : 'tls') : 'none'}
                          onChange={(e) => {
                            const val = e.target.value;
                            if (val === 'none') {
                              setEditTlsEnabled(false);
                              setEditRealityEnabled(false);
                            } else if (val === 'tls') {
                              setEditTlsEnabled(true);
                              setEditRealityEnabled(false);
                            } else {
                              setEditTlsEnabled(true);
                              setEditRealityEnabled(true);
                            }
                          }}
                        >
                          <option value="none">none</option>
                          <option value="tls">tls</option>
                          <option value="reality">reality</option>
                        </select>
                      </div>
                    </div>
                  </div>

                  {/* Network settings (tcp/ws/grpc) */}
                  <div className="editor-subcard">
                    <span className="editor-subcard-title">Network Settings ({editNetwork})</span>
                    {editNetwork === 'tcp' && (
                      <div className="form-group">
                        <label className="form-label">header*</label>
                        <select
                          className="select-input"
                          value={editHeaderType}
                          onChange={(e) => setEditHeaderType(e.target.value)}
                        >
                          <option value="">None</option>
                          <option value="http">http (HTTP Camouflage)</option>
                        </select>
                      </div>
                    )}
                    {(editNetwork === 'ws' || editNetwork === 'grpc' || editHeaderType === 'http') && (
                      <div className="editor-form-grid-2">
                        <div className="form-group">
                          <label className="form-label">Path*</label>
                          <input
                            type="text"
                            className="text-input"
                            placeholder="/graphql"
                            value={editPath}
                            onChange={(e) => setEditPath(e.target.value)}
                          />
                        </div>
                        <div className="form-group">
                          <label className="form-label">Host*</label>
                          <input
                            type="text"
                            className="text-input"
                            placeholder="google.com"
                            value={editHost}
                            onChange={(e) => setEditHost(e.target.value)}
                          />
                        </div>
                      </div>
                    )}
                  </div>

                  {/* TLS settings */}
                  {editTlsEnabled && (
                    <div className="editor-subcard">
                      <span className="editor-subcard-title">TLS Security Settings</span>
                      <div className="editor-form-grid-2" style={{ marginBottom: '14px' }}>
                        <div className="editor-checkbox-row" style={{ marginTop: '30px' }}>
                          <input
                            type="checkbox"
                            id="allowInsecure"
                            checked={editAllowInsecure}
                            onChange={(e) => setEditAllowInsecure(e.target.checked)}
                          />
                          <label htmlFor="allowInsecure" className="editor-checkbox-label">
                            Allow insecure*
                          </label>
                        </div>
                        <div className="form-group">
                          <label className="form-label">SNI*</label>
                          <input
                            type="text"
                            className="text-input"
                            placeholder="aka.ms"
                            value={editServerName}
                            onChange={(e) => setEditServerName(e.target.value)}
                          />
                        </div>
                      </div>
                      <div className="form-group">
                        <label className="form-label">ALPN* (Comma separated)</label>
                        <input
                          type="text"
                          className="text-input"
                          placeholder="h2, http/1.1"
                          value={editAlpn}
                          onChange={(e) => setEditAlpn(e.target.value)}
                        />
                      </div>
                    </div>
                  )}

                  {/* Reality camo settings */}
                  {editTlsEnabled && editRealityEnabled && (
                    <div className="editor-subcard">
                      <span className="editor-subcard-title">TLS Camouflage Settings</span>
                      <div className="form-group">
                        <label className="form-label">Fingerprint*</label>
                        <select
                          className="select-input"
                          value={editFingerprint}
                          onChange={(e) => setEditFingerprint(e.target.value)}
                        >
                          <option value="chrome">chrome</option>
                          <option value="firefox">firefox</option>
                          <option value="edge">edge</option>
                          <option value="safari">safari</option>
                          <option value="random">random</option>
                        </select>
                      </div>
                      <div className="form-group">
                        <label className="form-label">Reality Pbk* (Public Key)</label>
                        <input
                          type="text"
                          className="text-input"
                          placeholder="kovJ2h9HyJoofO83M00dYx8hzqeTBKyGQ3fs4Y2nMSE"
                          value={editPublicKey}
                          onChange={(e) => setEditPublicKey(e.target.value)}
                        />
                      </div>
                      <div className="form-group">
                        <label className="form-label">Reality Sid* (Short ID)</label>
                        <input
                          type="text"
                          className="text-input"
                          placeholder="97c4e5fcb1e8"
                          value={editShortId}
                          onChange={(e) => setEditShortId(e.target.value)}
                        />
                      </div>
                    </div>
                  )}
                </div>
              </div>

              {editSaveError && (
                <div style={{ background: 'rgba(239, 68, 68, 0.08)', border: '1px solid rgba(239, 68, 68, 0.2)', padding: '12px', borderRadius: '8px', fontSize: '13px', color: 'var(--status-error)', display: 'flex', gap: '8px', alignItems: 'center', marginTop: '20px' }}>
                  <ShieldAlert size={16} style={{ flexShrink: 0 }} />
                  <span style={{ wordBreak: 'break-word' }}>{editSaveError}</span>
                </div>
              )}
            </div>

            <footer className="editor-footer">
              <div className="editor-footer-left">
                <button
                  type="button"
                  className="btn btn-secondary"
                  onClick={() => {
                    // Quick default set for testing/reference
                    setEditTag('zoom-One-Piece');
                    setEditAddress('azureedge.duckdns.org');
                    setEditPort(443);
                    setEditProtocol('vless');
                    setEditUuid('a20d4836-4634-471b-b6c3-2f48acb0e9a5');
                    setEditFlow('');
                    setEditNetwork('tcp');
                    setEditTlsEnabled(true);
                    setEditRealityEnabled(true);
                    setEditServerName('aka.ms');
                    setEditFingerprint('chrome');
                    setEditPublicKey('kovJ2h9HyJoofO83M00dYx8hzqeTBKyGQ3fs4Y2nMSE');
                    setEditShortId('97c4e5fcb1e8');
                    setEditAllowInsecure(true);
                    pushSystemLog('Applied preset template options to editor fields.');
                  }}
                >
                  Apply settings to this group
                </button>
              </div>
              <div className="editor-footer-right">
                <button
                  type="button"
                  className="btn btn-secondary"
                  onClick={() => setIsEditingConfig(false)}
                >
                  Cancel
                </button>
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={handleSaveEditor}
                  disabled={isSavingConfig}
                  style={{ minWidth: '80px' }}
                >
                  {isSavingConfig ? (
                    <>
                      <RefreshCw size={14} className="spin" />
                      Validating...
                    </>
                  ) : (
                    'OK'
                  )}
                </button>
              </div>
            </footer>
          </div>
        </div>
      )}
    </div>
  );
}
