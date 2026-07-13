import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { useSettingsStore } from './settingsStore';
import { useProfileStore } from './profileStore';
import { useLogStore } from './logStore';
import { useToastStore } from './toastStore';
import { toUserFriendlyError } from '../utils/errors';

interface ConnectionState {
  isConnected: boolean;
  connectionStatus: 'connected' | 'connecting' | 'disconnected';
  uptime: number;
  httpPort: number;
  socksPort: number;
  mixedPort: number;
  uploadBytes: number;
  downloadBytes: number;
  uploadSpeed: number;
  downloadSpeed: number;
  activeConnections: number;
  speedHistory: { up: number; down: number }[];

  // Actions
  setPorts: (ports: { httpPort?: number; socksPort?: number; mixedPort?: number }) => void;
  setConnectionStatus: (status: 'connected' | 'connecting' | 'disconnected') => void;
  setIsConnected: (connected: boolean) => void;
  setUptime: (time: number) => void;
  updateTrafficStats: (stats: {
    uploadBytes: number;
    downloadBytes: number;
    uploadSpeed: number;
    downloadSpeed: number;
    activeConnections: number;
  }) => void;
  resetTrafficStats: () => void;
  toggleConnect: () => Promise<void>;
  requestElevation: () => Promise<void>;
}

export const useConnectionStore = create<ConnectionState>((set, get) => ({
  isConnected: false,
  connectionStatus: 'disconnected',
  uptime: 0,
  httpPort: 7890,
  socksPort: 7891,
  mixedPort: 7892,
  uploadBytes: 0,
  downloadBytes: 0,
  uploadSpeed: 0,
  downloadSpeed: 0,
  activeConnections: 0,
  speedHistory: [],

  setPorts: (ports) => set((state) => ({ ...state, ...ports })),
  setConnectionStatus: (status) => set({ connectionStatus: status }),
  setIsConnected: (connected) => set({ isConnected: connected }),
  setUptime: (time) => set({ uptime: time }),

  updateTrafficStats: (stats) => set((state) => ({
    ...state,
    ...stats,
    speedHistory: [...state.speedHistory, { up: stats.uploadSpeed, down: stats.downloadSpeed }].slice(-30)
  })),

  resetTrafficStats: () => set((state) => ({
    uploadSpeed: 0,
    downloadSpeed: 0,
    activeConnections: 0,
    speedHistory: [...state.speedHistory, { up: 0, down: 0 }].slice(-30)
  })),

  toggleConnect: async () => {
    const { connectionStatus, httpPort, socksPort, mixedPort } = get();
    const { settings, isElevated } = useSettingsStore.getState();
    const profileStore = useProfileStore.getState();
    const activeProfile = profileStore.activeProfile();
    const { selectedNodeTag } = profileStore;
    const logStore = useLogStore.getState();

    if (connectionStatus !== 'disconnected') {
      try {
        set({ connectionStatus: 'disconnected', isConnected: false });
        const result = await invoke<string>('toggle_proxy', { start: false, selectedOutboundTag: null });
        if (result === 'stopped') {
          logStore.pushSystemLog('Proxy tunnels stopped.');
        }
      } catch (e) {
        logStore.pushSystemLog(`Error stopping proxy: ${e}`);
      }
    } else {
      if (!activeProfile) {
        logStore.pushSystemLog('Error: Import a config first.');
        return;
      }

      let outboundTag = selectedNodeTag;
      if (!outboundTag) {
        const nodes = profileStore.nodes;
        if (nodes && nodes.length > 0) {
          outboundTag = nodes[0].tag;
          profileStore.selectNode(nodes[0]);
          logStore.pushSystemLog(`No node selected. Automatically selecting first node "${outboundTag}".`);
        } else {
          logStore.pushSystemLog('Error: Active profile has no nodes to connect to.');
          return;
        }
      }

      // TUN mode requires admin privileges — rollback to system proxy if not elevated
      if (settings.proxyMode === 'tun' && !isElevated) {
        logStore.pushSystemLog('TUN mode requires Administrator privileges. Not elevated. Rolling back to System Proxy mode...');
        
        const settingsStore = useSettingsStore.getState();
        await settingsStore.updateSettings({ proxyMode: 'system' });
        useToastStore.getState().addToast('warning', 'Switched to System Proxy mode (TUN requires Admin rights).', 'Fallback Activated');
        
        // Re-trigger connect in system proxy mode
        setTimeout(() => {
          get().toggleConnect();
        }, 500);
        return;
      }

      try {
        const conflict = await invoke<number | null>('check_port_conflict', { ports: [httpPort, socksPort, mixedPort] });
        if (conflict) {
          logStore.pushSystemLog(`Error: Port ${conflict} is in use.`);
          return;
        }
      } catch (err) {
        logStore.pushSystemLog(`Port check failed: ${err}. Proceeding anyway...`);
      }

      logStore.pushSystemLog(`Booting X-Link Core using "${activeProfile.name}" [${outboundTag}]...`);
      set({ connectionStatus: 'connecting' });

      try {
        const result = await invoke<string>('toggle_proxy', { start: true, selectedOutboundTag: outboundTag });
        if (result === 'started') {
          set({ isConnected: true, connectionStatus: 'connected' });
          logStore.pushSystemLog(`sing-box established on port Mixed:${mixedPort}.`);
          useToastStore.getState().addToast('success', `Secured via Mixed port ${mixedPort}`, 'Connection Established');
        }
      } catch (e) {
        set({ connectionStatus: 'disconnected' });
        logStore.pushSystemLog(`Startup error: ${e}`);
        useToastStore.getState().addToast('error', toUserFriendlyError(String(e)), 'Connection Failed');
      }
    }
  },

  requestElevation: async () => {
    const logStore = useLogStore.getState();
    try {
      logStore.pushSystemLog('Registering app to always run as Administrator on next launches...');
      try {
        await invoke('set_runas_admin', { enabled: true });
      } catch (err) {
        logStore.pushSystemLog(`Warning: Failed to set run-as-admin auto-elevation: ${err}`);
      }
      logStore.pushSystemLog('Requesting session elevation...');
      await invoke('request_elevation');
    } catch (e) {
      logStore.pushSystemLog(`Elevation aborted: ${e}`);
      useToastStore.getState().addToast('error', toUserFriendlyError(String(e)), 'Elevation Error');
    }
  }
}));
