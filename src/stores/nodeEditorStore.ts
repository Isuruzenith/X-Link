import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { useProfileStore } from './profileStore';
import { useLogStore } from './logStore';
import { useToastStore } from './toastStore';

interface NodeEditorState {
  isOpen: boolean;
  isSaving: boolean;
  saveError: string | null;
  section: 'basic' | 'transport' | 'tls';

  // Fields
  editTag: string;
  editAddress: string;
  editPort: number;
  editProtocol: string;
  editUuid: string;
  editFlow: string;
  editPassword: string;
  editMethod: string;
  editNetwork: string;
  editHeaderType: string;
  editPath: string;
  editHost: string;
  editServiceName: string;
  editTlsEnabled: boolean;
  editAllowInsecure: boolean;
  editServerName: string;
  editAlpn: string;
  editRealityEnabled: boolean;
  editFingerprint: string;
  editPublicKey: string;
  editShortId: string;
  editHy2Auth: string;
  editHy2UpBw: string;
  editHy2DownBw: string;
  editHy2ObfsType: string;
  editHy2ObfsPassword: string;
  editTuicUuid: string;
  editTuicPassword: string;
  editTuicCongestion: string;
  editTuicUdpMode: string;
  editSocksUser: string;
  editSocksPassword: string;
  editSocksVersion: number;
  editSocksUdpOverTcp: boolean;
  editHttpUser: string;
  editHttpPassword: string;
  editHttpTls: boolean;

  // Actions
  openEditor: (node: any) => void;
  closeEditor: () => void;
  setSection: (section: 'basic' | 'transport' | 'tls') => void;
  setField: (field: keyof Omit<NodeEditorState, 'isOpen' | 'isSaving' | 'saveError' | 'section' | 'openEditor' | 'closeEditor' | 'setSection' | 'setField' | 'saveEditor'>, value: any) => void;
  saveEditor: () => Promise<void>;
}

export const useNodeEditorStore = create<NodeEditorState>((set, get) => ({
  isOpen: false,
  isSaving: false,
  saveError: null,
  section: 'basic',

  // Fields default state
  editTag: '',
  editAddress: '',
  editPort: 443,
  editProtocol: 'vless',
  editUuid: '',
  editFlow: '',
  editPassword: '',
  editMethod: 'aes-256-gcm',
  editNetwork: 'tcp',
  editHeaderType: '',
  editPath: '',
  editHost: '',
  editServiceName: '',
  editTlsEnabled: false,
  editAllowInsecure: false,
  editServerName: '',
  editAlpn: '',
  editRealityEnabled: false,
  editFingerprint: 'chrome',
  editPublicKey: '',
  editShortId: '',
  editHy2Auth: '',
  editHy2UpBw: '100 mbps',
  editHy2DownBw: '200 mbps',
  editHy2ObfsType: '',
  editHy2ObfsPassword: '',
  editTuicUuid: '',
  editTuicPassword: '',
  editTuicCongestion: 'bbr',
  editTuicUdpMode: 'native',
  editSocksUser: '',
  editSocksPassword: '',
  editSocksVersion: 5,
  editSocksUdpOverTcp: false,
  editHttpUser: '',
  editHttpPassword: '',
  editHttpTls: false,

  openEditor: (node) => {
    const t = node.transport?.type || node.network || 'tcp';
    set({
      isOpen: true,
      saveError: null,
      section: 'basic',
      editTag: node.tag || '',
      editAddress: node.server || '',
      editPort: node.server_port || 443,
      editProtocol: node.type || 'vless',
      editUuid: node.uuid || node.id || '',
      editFlow: node.flow || '',
      editPassword: node.password || '',
      editMethod: node.method || 'aes-256-gcm',
      editHy2Auth: node.password || node.auth || '',
      editHy2UpBw: node.up_mbps ? `${node.up_mbps} mbps` : '100 mbps',
      editHy2DownBw: node.down_mbps ? `${node.down_mbps} mbps` : '200 mbps',
      editHy2ObfsType: node.obfs?.type || '',
      editHy2ObfsPassword: node.obfs?.password || '',
      editTuicUuid: node.uuid || '',
      editTuicPassword: node.password || '',
      editTuicCongestion: node.congestion_control || 'bbr',
      editTuicUdpMode: node.udp_relay_mode || 'native',
      editSocksUser: node.username || '',
      editSocksPassword: node.password || '',
      editSocksVersion: node.version || 5,
      editSocksUdpOverTcp: node.udp_over_tcp || false,
      editHttpUser: node.username || '',
      editHttpPassword: node.password || '',
      editHttpTls: !!node.tls?.enabled,
      editNetwork: ['ws', 'grpc', 'quic', 'http', 'httpupgrade'].includes(t) ? t : 'tcp',
      editPath: node.transport?.path || '',
      editHost: node.transport?.headers?.Host || '',
      editServiceName: node.transport?.service_name || '',
      editTlsEnabled: !!node.tls?.enabled,
      editAllowInsecure: !!node.tls?.insecure,
      editServerName: node.tls?.server_name || '',
      editAlpn: node.tls?.alpn?.join(', ') || '',
      editRealityEnabled: !!node.tls?.reality?.enabled,
      editFingerprint: node.tls?.utls?.fingerprint || 'chrome',
      editPublicKey: node.tls?.reality?.public_key || '',
      editShortId: node.tls?.reality?.short_id || '',
    });
  },

  closeEditor: () => set({ isOpen: false }),
  setSection: (section) => set({ section }),
  setField: (field, value) => set({ [field]: value } as any),

  saveEditor: async () => {
    const profileStore = useProfileStore.getState();
    const logStore = useLogStore.getState();
    const state = get();

    if (!profileStore.activeProfile()) return;
    set({ saveError: null, isSaving: true });

    if (!state.editTag.trim()) {
      set({ saveError: 'Tag name is required.', isSaving: false });
      return;
    }

    try {
      const outbound: any = { type: state.editProtocol, tag: state.editTag.trim() };

      if (['vless', 'vmess', 'trojan', 'shadowsocks', 'hysteria2', 'tuic', 'socks', 'http'].includes(state.editProtocol)) {
        outbound.server = state.editAddress.trim();
        outbound.server_port = Number(state.editPort) || 443;
      }

      if (state.editProtocol === 'vless' || state.editProtocol === 'vmess') {
        if (!state.editUuid.trim()) {
          set({ saveError: 'UUID required.', isSaving: false });
          return;
        }
        outbound.uuid = state.editUuid.trim();
        if (state.editFlow && state.editProtocol === 'vless') {
          outbound.flow = state.editFlow;
        }
      } else if (state.editProtocol === 'trojan') {
        outbound.password = state.editPassword.trim();
      } else if (state.editProtocol === 'shadowsocks') {
        outbound.method = state.editMethod || 'aes-256-gcm';
        outbound.password = state.editPassword.trim();
      } else if (state.editProtocol === 'hysteria2') {
        outbound.password = state.editHy2Auth.trim();
        if (state.editHy2UpBw) {
          outbound.up_mbps = parseFloat(state.editHy2UpBw) || undefined;
        }
        if (state.editHy2DownBw) {
          outbound.down_mbps = parseFloat(state.editHy2DownBw) || undefined;
        }
        if (state.editHy2ObfsType) {
          outbound.obfs = { type: state.editHy2ObfsType, password: state.editHy2ObfsPassword };
        }
      } else if (state.editProtocol === 'tuic') {
        outbound.uuid = state.editTuicUuid.trim();
        outbound.password = state.editTuicPassword.trim();
        outbound.congestion_control = state.editTuicCongestion;
        outbound.udp_relay_mode = state.editTuicUdpMode;
      } else if (state.editProtocol === 'socks') {
        outbound.version = state.editSocksVersion;
        if (state.editSocksUser) {
          outbound.username = state.editSocksUser;
          outbound.password = state.editSocksPassword;
        }
        outbound.udp_over_tcp = state.editSocksUdpOverTcp;
      } else if (state.editProtocol === 'http') {
        if (state.editHttpUser) {
          outbound.username = state.editHttpUser;
          outbound.password = state.editHttpPassword;
        }
        if (state.editHttpTls) outbound.tls = { enabled: true };
      }

      const needTls = state.editTlsEnabled || ['hysteria2', 'tuic'].includes(state.editProtocol);
      if (needTls && !['socks', 'http', 'shadowsocks'].includes(state.editProtocol)) {
        const tls: any = { enabled: true };
        if (state.editServerName.trim()) tls.server_name = state.editServerName.trim();
        if (state.editAllowInsecure) tls.insecure = true;
        if (state.editAlpn.trim()) tls.alpn = state.editAlpn.split(',').map((s) => s.trim()).filter(Boolean);
        if (state.editFingerprint) tls.utls = { enabled: true, fingerprint: state.editFingerprint };
        if (state.editRealityEnabled && !['hysteria2', 'tuic'].includes(state.editProtocol)) {
          if (!state.editPublicKey.trim()) {
            set({ saveError: 'Reality Public Key required.', isSaving: false });
            return;
          }
          tls.reality = { enabled: true, public_key: state.editPublicKey.trim(), short_id: state.editShortId.trim() || undefined };
        }
        outbound.tls = tls;
      }

      if (state.editNetwork && state.editNetwork !== 'tcp' && !['socks', 'http', 'hysteria2', 'tuic'].includes(state.editProtocol)) {
        const transport: any = { type: state.editNetwork };
        if (state.editPath.trim()) transport.path = state.editPath.trim();
        if (state.editHost.trim()) transport.headers = { Host: state.editHost.trim() };
        if (state.editServiceName.trim()) transport.service_name = state.editServiceName.trim();
        outbound.transport = transport;
      }

      await invoke('update_node', { newOutbound: outbound });
      const outbounds = await invoke<any[]>('get_config_outbounds');
      profileStore.updateNodesList(outbounds || []);

      if (state.editTag.trim() !== profileStore.selectedNodeTag && profileStore.selectedNodeTag) {
        const active = await invoke<any>('get_active_outbound').catch(() => null);
        if (active) profileStore.selectNode({ tag: active.tag });
      } else {
        profileStore.selectNode({ tag: outbound.tag });
      }

      logStore.pushSystemLog(`Node "${outbound.tag}" updated and validated.`);
      useToastStore.getState().addToast('success', `Server "${outbound.tag}" updated successfully.`, 'Server Saved');
      set({ isOpen: false });
    } catch (err) {
      set({ saveError: String(err) });
      useToastStore.getState().addToast('error', `Failed to save server: ${err}`, 'Save Error');
    } finally {
      set({ isSaving: false });
    }
  }
}));
