import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { useProfileStore } from './profileStore';
import { useLogStore } from './logStore';
import { useToastStore } from './toastStore';
import type { ProxyNode } from '../utils/store';

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
  editWgPrivateKey: string;
  editWgAddress: string;
  editWgPeerPublicKey: string;
  editWgPsk: string;
  editWgMtu: number;
  editWgReserved: string;
  editSshUser: string;
  editSshPassword: string;
  editSshPrivateKey: string;
  editSshHostKey: string;

  // Actions
  openEditor: (node?: ProxyNode | null) => void;
  closeEditor: () => void;
  setSection: (section: 'basic' | 'transport' | 'tls') => void;
  setField: (field: keyof Omit<NodeEditorState, 'isOpen' | 'isSaving' | 'saveError' | 'section' | 'openEditor' | 'closeEditor' | 'setSection' | 'setField' | 'saveEditor'>, value: unknown) => void;
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
  editAlpn: 'http/1.1',
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
  editWgPrivateKey: '',
  editWgAddress: '10.0.0.2/32',
  editWgPeerPublicKey: '',
  editWgPsk: '',
  editWgMtu: 1420,
  editWgReserved: '',
  editSshUser: 'root',
  editSshPassword: '',
  editSshPrivateKey: '',
  editSshHostKey: '',

  openEditor: (node) => {
    if (!node) {
      const nodeCount = useProfileStore.getState().nodes.length;
      set({
        isOpen: true,
        saveError: null,
        section: 'basic',
        editTag: `Server ${nodeCount + 1}`,
        editAddress: '',
        editPort: 443,
        editProtocol: 'vless',
        editUuid: typeof crypto !== 'undefined' && crypto.randomUUID ? crypto.randomUUID() : '',
        editFlow: '',
        editPassword: '',
        editMethod: 'aes-256-gcm',
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
        editNetwork: 'tcp',
        editPath: '',
        editHost: '',
        editServiceName: '',
        editTlsEnabled: false,
        editAllowInsecure: false,
        editServerName: '',
        editAlpn: 'http/1.1',
        editRealityEnabled: false,
        editFingerprint: 'chrome',
        editPublicKey: '',
        editShortId: '',
        editWgPrivateKey: '',
        editWgAddress: '10.0.0.2/32',
        editWgPeerPublicKey: '',
        editWgPsk: '',
        editWgMtu: 1420,
        editWgReserved: '',
        editSshUser: 'root',
        editSshPassword: '',
        editSshPrivateKey: '',
        editSshHostKey: '',
      });
      return;
    }

    const t = node.transport?.type || node.network || 'tcp';
    const peer = node.peers && node.peers.length > 0 ? node.peers[0] : null;
    const peerServer = peer?.server || node.server || '';
    const peerPort = peer?.server_port || node.server_port || (node.type === 'wireguard' ? 51820 : node.type === 'ssh' ? 22 : 443);
    const peerPub = peer?.public_key || (node.peer_public_key as string) || '';
    const peerPsk = peer?.pre_shared_key || (node.pre_shared_key as string) || '';
    const reservedArr = peer?.reserved || (node.reserved as number[]) || [];
    const localAddrArr = (node.local_address as string[]) || (node.address as string[]) || [];

    set({
      isOpen: true,
      saveError: null,
      section: 'basic',
      editTag: node.tag || '',
      editAddress: peerServer,
      editPort: peerPort,
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
      editWgPrivateKey: (node.private_key as string) || '',
      editWgAddress: localAddrArr.length > 0 ? localAddrArr.join(', ') : '10.0.0.2/32',
      editWgPeerPublicKey: peerPub,
      editWgPsk: peerPsk,
      editWgMtu: (node.mtu as number) || 1420,
      editWgReserved: reservedArr.length > 0 ? JSON.stringify(reservedArr) : '',
      editSshUser: (node.user as string) || 'root',
      editSshPassword: node.password || '',
      editSshPrivateKey: (node.private_key as string) || (node.private_key_path as string) || '',
      editSshHostKey: Array.isArray(node.host_key) ? node.host_key.join('\n') : (typeof node.host_key === 'string' ? node.host_key : ''),
      editNetwork: ['ws', 'grpc', 'quic', 'http', 'httpupgrade'].includes(t) ? t : 'tcp',
      editPath: node.transport?.path || '',
      editHost: node.transport?.host || node.transport?.headers?.Host || node.transport?.headers?.host || '',
      editServiceName: node.transport?.service_name || '',
      editTlsEnabled: !!node.tls?.enabled,
      editAllowInsecure: !!node.tls?.insecure,
      editServerName: node.tls?.server_name || '',
      editAlpn: node.tls?.alpn
        ? node.tls.alpn.map((s) => s.trim()).filter(Boolean).sort().join(', ')
        : 'http/1.1',
      editRealityEnabled: !!node.tls?.reality?.enabled,
      editFingerprint: node.tls?.utls?.fingerprint || 'chrome',
      editPublicKey: node.tls?.reality?.public_key || '',
      editShortId: node.tls?.reality?.short_id || '',
    });
  },

  closeEditor: () => set({ isOpen: false }),
  setSection: (section) => set({ section }),
  setField: (field, value) => set({ [field]: value } as unknown as Partial<NodeEditorState>),

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
      const outbound: Partial<ProxyNode> = { type: state.editProtocol, tag: state.editTag.trim() };

      if (['vless', 'vmess', 'trojan', 'shadowsocks', 'hysteria2', 'tuic', 'socks', 'http', 'ssh'].includes(state.editProtocol)) {
        outbound.server = state.editAddress.trim();
        outbound.server_port = Number(state.editPort) || (state.editProtocol === 'ssh' ? 22 : 443);
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
      } else if (state.editProtocol === 'wireguard') {
        if (!state.editWgPrivateKey.trim()) {
          set({ saveError: 'WireGuard private key is required.', isSaving: false });
          return;
        }
        if (!state.editWgPeerPublicKey.trim()) {
          set({ saveError: 'Peer public key is required.', isSaving: false });
          return;
        }
        let parsedReserved: number[] | undefined;
        if (state.editWgReserved.trim()) {
          try {
            parsedReserved = JSON.parse(state.editWgReserved.trim());
          } catch {
            set({ saveError: 'Reserved bytes must be a valid JSON array, e.g. [0,0,0]', isSaving: false });
            return;
          }
        }
        outbound.server = state.editAddress.trim();
        outbound.server_port = Number(state.editPort) || 51820;
        outbound.private_key = state.editWgPrivateKey.trim();
        outbound.peer_public_key = state.editWgPeerPublicKey.trim();
        outbound.local_address = state.editWgAddress.split(',').map((s) => s.trim()).filter(Boolean);
        outbound.pre_shared_key = state.editWgPsk.trim() || undefined;
        outbound.mtu = Number(state.editWgMtu) || 1420;
        outbound.reserved = parsedReserved;
      } else if (state.editProtocol === 'ssh') {
        if (!state.editSshUser.trim()) {
          set({ saveError: 'SSH username is required.', isSaving: false });
          return;
        }
        outbound.user = state.editSshUser.trim();
        if (state.editSshPassword.trim()) {
          outbound.password = state.editSshPassword.trim();
        }
        if (state.editSshPrivateKey.trim()) {
          outbound.private_key = state.editSshPrivateKey.trim();
        }
        if (state.editSshHostKey.trim()) {
          outbound.host_key = state.editSshHostKey.split('\n').map((s) => s.trim()).filter(Boolean);
        }
      }

      const needTls = state.editTlsEnabled || ['hysteria2', 'tuic'].includes(state.editProtocol);
      if (needTls && !['socks', 'http', 'shadowsocks', 'wireguard', 'ssh'].includes(state.editProtocol)) {
        const tls: NonNullable<ProxyNode['tls']> = { enabled: true };
        if (state.editServerName.trim()) tls.server_name = state.editServerName.trim();
        if (state.editAllowInsecure) tls.insecure = true;
        if (state.editAlpn.trim()) tls.alpn = state.editAlpn.split(',').map((s) => s.trim()).filter(Boolean);
        if (state.editFingerprint) tls.utls = { fingerprint: state.editFingerprint };
        if (state.editRealityEnabled && !['hysteria2', 'tuic'].includes(state.editProtocol)) {
          if (!state.editPublicKey.trim()) {
            set({ saveError: 'Reality Public Key required.', isSaving: false });
            return;
          }
          tls.reality = { enabled: true, public_key: state.editPublicKey.trim(), short_id: state.editShortId.trim() || undefined };
        }
        outbound.tls = tls;
      }

      if (state.editNetwork && state.editNetwork !== 'tcp' && !['socks', 'http', 'hysteria2', 'tuic', 'wireguard', 'ssh'].includes(state.editProtocol)) {
        const transport: NonNullable<ProxyNode['transport']> = { type: state.editNetwork };
        if (state.editPath.trim()) transport.path = state.editPath.trim();
        if (state.editHost.trim()) {
          if (state.editNetwork === 'httpupgrade') transport.host = state.editHost.trim();
          else transport.headers = { Host: state.editHost.trim() };
        }
        if (state.editServiceName.trim()) transport.service_name = state.editServiceName.trim();
        outbound.transport = transport;
      }

      await invoke('update_node', { newOutbound: outbound });
      const outbounds = await invoke<ProxyNode[]>('get_config_outbounds');
      profileStore.updateNodesList(outbounds || []);

      if (state.editTag.trim() !== profileStore.selectedNodeTag && profileStore.selectedNodeTag) {
        const active = await invoke<ProxyNode>('get_active_outbound').catch(() => null);
        if (active) profileStore.selectNode({ tag: active.tag });
      } else {
        profileStore.selectNode({ tag: state.editTag.trim() });
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
