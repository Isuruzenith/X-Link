import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { useSettingsStore } from './settingsStore';
import { useProfileStore } from './profileStore';
import { useLogStore } from './logStore';
import { useToastStore } from './toastStore';

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

      // TUN mode requires admin privileges — auto-trigger elevation if not already elevated
      if (settings.proxyMode === 'tun' && !isElevated) {
        logStore.pushSystemLog('TUN mode requires Administrator privileges. Requesting elevation...');
        try {
          await invoke('request_elevation');
        } catch {
          logStore.pushSystemLog(`Elevation denied — cannot start TUN mode without admin rights.`);
        }
        return;
      }

      try {
        const conflict = await invoke<number | null>('check_port_conflict', { ports: [httpPort, socksPort, mixedPort] });
        if (conflict) {
          logStore.pushSystemLog(`Error: Port ${conflict} is in use.`);
          return;
        }
      } catch {}

      logStore.pushSystemLog(`Booting X-Link Core using "${activeProfile.name}"...`);
      set({ connectionStatus: 'connecting' });

      try {
        const result = await invoke<string>('toggle_proxy', { start: true, selectedOutboundTag: selectedNodeTag });
        if (result === 'started') {
          set({ isConnected: true, connectionStatus: 'connected' });
          logStore.pushSystemLog(`sing-box established on port Mixed:${mixedPort}.`);
          useToastStore.getState().addToast('success', `Connected via Mixed port ${mixedPort}`);
        }
      } catch (e) {
        set({ connectionStatus: 'disconnected' });
        logStore.pushSystemLog(`Startup error: ${e}`);
        useToastStore.getState().addToast('error', `Connection failed: ${e}`);
      }
    }
  },

  requestElevation: async () => {
    const logStore = useLogStore.getState();
    try {
      await invoke('request_elevation');
    } catch (e) {
      logStore.pushSystemLog(`Elevation aborted: ${e}`);
    }
  }
}));
