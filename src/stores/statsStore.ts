import { create } from 'zustand';
import { storeHelper, type NodeUsageStats } from '../utils/store';

let saveTimeout: any = null;
const debouncedSave = (stats: Record<string, NodeUsageStats>) => {
  if (saveTimeout) clearTimeout(saveTimeout);
  saveTimeout = setTimeout(async () => {
    await storeHelper.saveStats(stats);
  }, 5000);
};

interface StatsState {
  stats: Record<string, NodeUsageStats>;
  initStats: () => Promise<void>;
  recordTraffic: (nodeId: string, deltaUpload: number, deltaDownload: number) => Promise<void>;
  recordNewConnection: (nodeId: string) => Promise<void>;
  resetStats: () => Promise<void>;
}

export const useStatsStore = create<StatsState>((set, get) => ({
  stats: {},

  initStats: async () => {
    const saved = await storeHelper.getStats();
    set({ stats: saved });
  },

  recordTraffic: async (nodeId, deltaUpload, deltaDownload) => {
    if (!nodeId) return;
    if (deltaUpload === 0 && deltaDownload === 0) return;
    
    const currentStats = { ...get().stats };
    const now = Date.now();
    const nodeStats = currentStats[nodeId] || {
      totalUploadBytes: 0,
      totalDownloadBytes: 0,
      connectionCount: 0,
      firstUsedAt: now,
    };

    const updatedNodeStats: NodeUsageStats = {
      ...nodeStats,
      totalUploadBytes: nodeStats.totalUploadBytes + deltaUpload,
      totalDownloadBytes: nodeStats.totalDownloadBytes + deltaDownload,
      firstUsedAt: nodeStats.firstUsedAt || now,
      lastUsedAt: now,
    };

    currentStats[nodeId] = updatedNodeStats;
    set({ stats: currentStats });
    debouncedSave(currentStats);
  },

  recordNewConnection: async (nodeId) => {
    if (!nodeId) return;
    
    const currentStats = { ...get().stats };
    const now = Date.now();
    const nodeStats = currentStats[nodeId] || {
      totalUploadBytes: 0,
      totalDownloadBytes: 0,
      connectionCount: 0,
      firstUsedAt: now,
    };

    const updatedNodeStats: NodeUsageStats = {
      ...nodeStats,
      connectionCount: nodeStats.connectionCount + 1,
      firstUsedAt: nodeStats.firstUsedAt || now,
      lastUsedAt: now,
    };

    currentStats[nodeId] = updatedNodeStats;
    set({ stats: currentStats });
    debouncedSave(currentStats);
  },

  resetStats: async () => {
    if (saveTimeout) clearTimeout(saveTimeout);
    set({ stats: {} });
    await storeHelper.saveStats({});
  },
}));
