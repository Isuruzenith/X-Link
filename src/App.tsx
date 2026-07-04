import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { NavRail } from './components/NavRail';
import type { TabId } from './components/NavRail';
import { DashboardView } from './views/DashboardView';
import { ConfigView } from './views/ConfigView';
import { RoutingView } from './views/RoutingView';
import { LogsView } from './views/LogsView';
import { SettingsView } from './views/SettingsView';
import { ConnectionsView } from './views/ConnectionsView.tsx';
import { NodeEditor } from './components/domain/NodeEditor';
import { ToastContainer } from './components/ToastContainer';
import { ErrorBoundary } from './components/ErrorBoundary';
import { migrateStores } from './utils/migrate';

import { useSettingsStore } from './stores/settingsStore';
import { useConnectionStore } from './stores/connectionStore';
import { useProfileStore } from './stores/profileStore';
import { useRoutingStore } from './stores/routingStore';
import { useLogStore } from './stores/logStore';
import { useStatsStore } from './stores/statsStore';

export default function App() {
  const [activeTab, setActiveTab] = useState<TabId>('dashboard');
  const hasAutoConnected = useRef(false);
  const prevUploadRef = useRef(0);
  const prevDownloadRef = useRef(0);
  const connectionCountedRef = useRef(false);

  // Zustand Store hooks
  const { initSettings, checkElevated, fetchVersions, setConflictingPorts } = useSettingsStore();
  const { initProfiles, refreshNodes } = useProfileStore();
  const { initRouting } = useRoutingStore();
  const { pushLog, pushSystemLog } = useLogStore();
  const { initStats, recordTraffic, recordNewConnection } = useStatsStore();
  const {
    setIsConnected,
    setConnectionStatus,
    setUptime,
    updateTrafficStats,
    resetTrafficStats,
    setPorts,
    mixedPort,
    httpPort,
    socksPort
  } = useConnectionStore();

  // ── INITIALIZATION ─────────────────────────────────────────────────────────
  useEffect(() => {
    (async () => {
      // Run schema migration before any store reads
      await migrateStores();

      // Parallel initialization of stores and system parameters
      try {
        await Promise.all([
          initSettings(),
          checkElevated(),
          initProfiles(),
          initRouting(),
          initStats(),
        ]);
      } catch (e) {
        console.error('[App] Failed to initialize stores:', e);
      }

      // Show the window once stores are initialized and UI is ready
      invoke('show_window').catch(() => {});

      // Fetch versions in the background to prevent blocking app startup
      fetchVersions().catch(() => {});

      const updatedSettings = useSettingsStore.getState().settings;
      const elevated = useSettingsStore.getState().isElevated;
      const activeProfile = useProfileStore.getState().activeProfile();

      // Sync ports in connectionStore with settings
      setPorts({
        httpPort: updatedSettings.httpPort,
        socksPort: updatedSettings.socksPort,
        mixedPort: updatedSettings.mixedPort
      });

      let resolvedTag: string | null = null;
      if (activeProfile) {
        await refreshNodes(activeProfile.id);
        resolvedTag = useProfileStore.getState().selectedNodeTag;
      }

      // Auto-connect if elevated
      if (elevated && activeProfile && !hasAutoConnected.current) {
        hasAutoConnected.current = true;
        (async () => {
          try {
            const status = await invoke<{ status: string }>('get_proxy_status');
            if (status.status === 'disconnected') {
              const conflict = await invoke<number | null>('check_port_conflict', {
                ports: [updatedSettings.httpPort, updatedSettings.socksPort, updatedSettings.mixedPort]
              });
              if (!conflict) {
                pushSystemLog('Administrator privileges detected. Auto-connecting...');
                setConnectionStatus('connecting');
                const result = await invoke<string>('toggle_proxy', { start: true, selectedOutboundTag: resolvedTag });
                if (result === 'started') {
                  setIsConnected(true);
                  setConnectionStatus('connected');
                  pushSystemLog(`sing-box established automatically on Mixed port ${updatedSettings.mixedPort}.`);
                } else {
                  setConnectionStatus('disconnected');
                }
              } else {
                pushSystemLog(`Auto-connect skipped: Port ${conflict} is in use.`);
              }
            }
          } catch (e) {
            pushSystemLog(`Auto-connect error: ${e}`);
          }
        })();
      }
    })();

    pushSystemLog('X-Link Core initialized.');
    pushSystemLog('Ready to establish proxy tunnels.');

    // Listen to sing-box logs emitted by Rust backend
    const unlistenLog = listen<string>('sing-box-log', (e) => {
      let line = e.payload.trim();
      if (!line) return;
      // eslint-disable-next-line no-control-regex
      const ansiRegex = new RegExp('\\x1B(?:[@-Z\\\\-_]|\\[[0-?]*[ -/]*[@-~])', 'g');
      line = line.replace(ansiRegex, '');
      let type: 'info' | 'warn' | 'error' | 'system' = 'info';
      if (line.toLowerCase().includes('warn')) {
        type = 'warn';
      } else if (line.toLowerCase().includes('err') || line.toLowerCase().includes('fatal')) {
        // Downgrade harmless normal/idle WebSocket, telemetry closures, and database warnings to 'info' to avoid user panic
        const isHarmlessClose = 
          line.toLowerCase().includes('ws closed: 1000') ||
          line.toLowerCase().includes('wsarecv: an existing connection was forcibly closed') ||
          line.toLowerCase().includes('wsarecv: a connection attempt failed') ||
          line.toLowerCase().includes('connection download closed: raw-read') ||
          line.toLowerCase().includes('failed to initialize geosite') ||
          line.toLowerCase().includes('failed to initialize geoip');
          
        if (isHarmlessClose) {
          type = 'info';
        } else {
          type = 'error';
        }
      }
      pushLog(type, line);
    });

    // Listen to process terminations
    const unlistenTerm = listen<number | null>('sing-box-terminated', (e) => {
      setIsConnected(false);
      setConnectionStatus('disconnected');
      pushSystemLog(`sing-box terminated (code ${e.payload ?? 'None'})`);
    });

    // Listen to status changes from the backend (hot-reloads, swaps)
    const unlistenStatus = listen<string>('proxy-status-changed', (e) => {
      const status = e.payload as 'connected' | 'connecting' | 'disconnected';
      setConnectionStatus(status);
      setIsConnected(status === 'connected');
    });

    // Listen to settings changes from system tray mode switching
    const unlistenSettings = listen('settings-changed', () => {
      initSettings();
    });

    // Fetch initial log buffer from Rust sidecar manager
    (async () => {
      try {
        const buffered = await invoke<string[]>('get_buffered_logs');
        if (buffered?.length) {
          buffered.forEach((line) => {
            let type: 'info' | 'warn' | 'error' | 'system' = 'info';
            if (line.toLowerCase().includes('warn')) type = 'warn';
            else if (line.toLowerCase().includes('err')) type = 'error';
            pushLog(type, line);
          });
        }
      } catch {
        // ignore
      }
    })();

    return () => {
      unlistenLog.then((f) => f());
      unlistenTerm.then((f) => f());
      unlistenStatus.then((f) => f());
      unlistenSettings.then((f) => f());
    };
  }, [checkElevated, fetchVersions, initProfiles, initRouting, initSettings, initStats, pushLog, pushSystemLog, refreshNodes, setConnectionStatus, setIsConnected, setPorts]);

  // ── PORT CONFLICT DETECTOR ─────────────────────────────────────────────────
  useEffect(() => {
    (async () => {
      const ports = [httpPort, socksPort, mixedPort];
      const conflicts: number[] = [];
      for (const p of ports) {
        try {
          const res = await invoke<number | null>('check_port_conflict', { ports: [p] });
          if (res) conflicts.push(p);
        } catch {
          // ignore
        }
      }
      setConflictingPorts(conflicts);
    })();
  }, [activeTab, httpPort, socksPort, mixedPort, setConflictingPorts]);

  // ── TRAFFIC & STATUS POLLING ───────────────────────────────────────────────
  useEffect(() => {
    const poll = setInterval(async () => {
      try {
        const status = await invoke<{
          status: 'connected' | 'connecting' | 'disconnected';
          httpPort: number;
          socksPort: number;
          mixedPort: number;
          tunEnabled: boolean;
          uptime: number;
        }>('get_proxy_status');
        
        const active = status.status === 'connected';
        setIsConnected(active);
        setConnectionStatus(status.status);
        setUptime(status.uptime);
        
        if (active) {
          const stats = await invoke<{
            uploadBytes: number;
            downloadBytes: number;
            uploadSpeed: number;
            downloadSpeed: number;
            activeConnections: number;
          }>('get_traffic_stats');
          updateTrafficStats(stats);

          // Calculate deltas and update stats store
          const currentUpload = stats.uploadBytes;
          const currentDownload = stats.downloadBytes;
          
          const deltaUpload = prevUploadRef.current > 0 && currentUpload >= prevUploadRef.current
            ? currentUpload - prevUploadRef.current
            : 0;
          const deltaDownload = prevDownloadRef.current > 0 && currentDownload >= prevDownloadRef.current
            ? currentDownload - prevDownloadRef.current
            : 0;

          prevUploadRef.current = currentUpload;
          prevDownloadRef.current = currentDownload;

          const activeNodeTag = useProfileStore.getState().selectedNodeTag;
          if (activeNodeTag) {
            if (deltaUpload > 0 || deltaDownload > 0) {
              recordTraffic(activeNodeTag, deltaUpload, deltaDownload).catch(() => {});
            }
            if (!connectionCountedRef.current) {
              recordNewConnection(activeNodeTag).catch(() => {});
              connectionCountedRef.current = true;
            }
          }
        } else {
          resetTrafficStats();
          prevUploadRef.current = 0;
          prevDownloadRef.current = 0;
          connectionCountedRef.current = false;
        }
      } catch {
        // ignore
      }
    }, 1000);
    return () => clearInterval(poll);
  }, [setIsConnected, setConnectionStatus, setUptime, updateTrafficStats, resetTrafficStats, recordTraffic, recordNewConnection]);

  return (
    <div className="app-root">
      <NavRail activeTab={activeTab} onTabChange={setActiveTab} />
      
      <div className="view-host">
        {activeTab === 'dashboard' && <ErrorBoundary fallbackTitle="Dashboard Error"><DashboardView onNavigateToTab={setActiveTab} /></ErrorBoundary>}
        {activeTab === 'profiles' && <ErrorBoundary fallbackTitle="Profiles Error"><ConfigView /></ErrorBoundary>}
        {activeTab === 'routing' && <ErrorBoundary fallbackTitle="Routing Error"><RoutingView /></ErrorBoundary>}
        {activeTab === 'connections' && <ErrorBoundary fallbackTitle="Connections Monitor Error"><ConnectionsView /></ErrorBoundary>}
        {activeTab === 'logs' && <ErrorBoundary fallbackTitle="Logs Error"><LogsView /></ErrorBoundary>}
        {activeTab === 'settings' && <ErrorBoundary fallbackTitle="Settings Error"><SettingsView /></ErrorBoundary>}
      </div>

      <NodeEditor />
      <ToastContainer />
    </div>
  );
}
