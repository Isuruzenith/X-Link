// Wiring of the pure applier (applyPolicy.ts) to the native cold restart and the
// app stores. Shared by the settings and routing stores.
import { invoke } from '@tauri-apps/api/core';
import { createApplier, type ApplyOutcome } from './applyPolicy';
import { useConnectionStore } from '../stores/connectionStore';
import { useProfileStore } from '../stores/profileStore';
import { useToastStore } from '../stores/toastStore';
import { useLogStore } from '../stores/logStore';

export const APPLY_DEBOUNCE_MS = 800;

export const connectedApplyDeps = {
  debounceMs: APPLY_DEBOUNCE_MS,
  isConnected: () => useConnectionStore.getState().connectionStatus === 'connected',
  isRunning: async () => {
    const status = await invoke<{ status: string }>('get_proxy_status');
    return status.status === 'connected';
  },
  reload: async () => {
    await invoke('reload_active_config', {
      selectedOutboundTag: useProfileStore.getState().selectedNodeTag,
    });
  },
};

async function syncConnectionStatus() {
  try {
    const status = await invoke<{
      status: 'connected' | 'connecting' | 'disconnected';
      effectiveProxyMode: 'tun' | 'system' | null;
    }>('get_proxy_status');
    const connection = useConnectionStore.getState();
    connection.setConnectionStatus(status.status);
    connection.setIsConnected(status.status === 'connected');
    connection.setEffectiveProxyMode(status.effectiveProxyMode);
  } catch { /* leave the last known state */ }
}

export function reportApplyOutcome(what: string) {
  return (outcome: ApplyOutcome, detail?: string) => {
    const toast = useToastStore.getState().addToast;
    const log = useLogStore.getState().pushSystemLog;
    if (detail) log(`[Apply] ${what}: ${outcome}: ${detail}`);
    switch (outcome) {
      case 'applied':
        toast('success', `${what} applied with a validated restart.`, 'Changes Applied');
        break;
      case 'invalid':
        toast('warning', `${detail}. Changes are saved and will apply once valid.`, 'Not Applied');
        break;
      case 'kept-running':
        toast('error', `${what} could not be applied; previous values were restored and the connection kept running.`, 'Apply Failed');
        break;
      case 'recovered':
        toast('error', `${what} could not be applied; previous values were restored and the connection was recovered.`, 'Apply Failed');
        break;
      case 'recovery-failed':
        toast('error', `${what} could not be applied and the previous connection could not be recovered. Previous values were restored; reconnect manually.`, 'Apply Failed');
        void syncConnectionStatus();
        break;
      case 'skipped':
        break;
    }
  };
}

export { createApplier };
