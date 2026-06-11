import React from 'react';
import { X, RefreshCw, ShieldAlert, Info } from 'lucide-react';

const PROTOCOL_LABELS: Record<string, string> = {
  vless: 'VLESS', vmess: 'VMess', trojan: 'Trojan',
  shadowsocks: 'Shadowsocks', hysteria2: 'Hysteria2', tuic: 'TUIC v5',
  wireguard: 'WireGuard', ssh: 'SSH', socks: 'SOCKS5', http: 'HTTP',
};

interface NodeEditorProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: () => void;
  isSaving: boolean;
  saveError: string | null;
  section: 'basic' | 'transport' | 'tls';
  onSectionChange: (s: 'basic' | 'transport' | 'tls') => void;
  onApplyPreset: () => void;
  // All the edit state
  editTag: string; setEditTag: (v: string) => void;
  editAddress: string; setEditAddress: (v: string) => void;
  editPort: number; setEditPort: (v: number) => void;
  editProtocol: string; setEditProtocol: (v: string) => void;
  editUuid: string; setEditUuid: (v: string) => void;
  editFlow: string; setEditFlow: (v: string) => void;
  editPassword: string; setEditPassword: (v: string) => void;
  editMethod: string; setEditMethod: (v: string) => void;
  editNetwork: string; setEditNetwork: (v: string) => void;
  editHeaderType: string; setEditHeaderType: (v: string) => void;
  editPath: string; setEditPath: (v: string) => void;
  editHost: string; setEditHost: (v: string) => void;
  editServiceName: string; setEditServiceName: (v: string) => void;
  editTlsEnabled: boolean; setEditTlsEnabled: (v: boolean) => void;
  editAllowInsecure: boolean; setEditAllowInsecure: (v: boolean) => void;
  editServerName: string; setEditServerName: (v: string) => void;
  editAlpn: string; setEditAlpn: (v: string) => void;
  editRealityEnabled: boolean; setEditRealityEnabled: (v: boolean) => void;
  editFingerprint: string; setEditFingerprint: (v: string) => void;
  editPublicKey: string; setEditPublicKey: (v: string) => void;
  editShortId: string; setEditShortId: (v: string) => void;
  editHy2Auth: string; setEditHy2Auth: (v: string) => void;
  editHy2UpBw: string; setEditHy2UpBw: (v: string) => void;
  editHy2DownBw: string; setEditHy2DownBw: (v: string) => void;
  editHy2ObfsType: string; setEditHy2ObfsType: (v: string) => void;
  editHy2ObfsPassword: string; setEditHy2ObfsPassword: (v: string) => void;
  editTuicUuid: string; setEditTuicUuid: (v: string) => void;
  editTuicPassword: string; setEditTuicPassword: (v: string) => void;
  editTuicCongestion: string; setEditTuicCongestion: (v: string) => void;
  editTuicUdpMode: string; setEditTuicUdpMode: (v: string) => void;
  editWgSecretKey: string; setEditWgSecretKey: (v: string) => void;
  editWgPeerPublicKey: string; setEditWgPeerPublicKey: (v: string) => void;
  editWgPreSharedKey: string; setEditWgPreSharedKey: (v: string) => void;
  editWgEndpoint: string; setEditWgEndpoint: (v: string) => void;
  editWgAllowedIps: string; setEditWgAllowedIps: (v: string) => void;
  editWgReserved: string; setEditWgReserved: (v: string) => void;
  editWgMtu: number; setEditWgMtu: (v: number) => void;
  editSshUser: string; setEditSshUser: (v: string) => void;
  editSshPassword: string; setEditSshPassword: (v: string) => void;
  editSshPrivateKey: string; setEditSshPrivateKey: (v: string) => void;
  editSshHostKey: string; setEditSshHostKey: (v: string) => void;
  editSocksUser: string; setEditSocksUser: (v: string) => void;
  editSocksPassword: string; setEditSocksPassword: (v: string) => void;
  editSocksVersion: number; setEditSocksVersion: (v: number) => void;
  editSocksUdpOverTcp: boolean; setEditSocksUdpOverTcp: (v: boolean) => void;
  editHttpUser: string; setEditHttpUser: (v: string) => void;
  editHttpPassword: string; setEditHttpPassword: (v: string) => void;
  editHttpTls: boolean; setEditHttpTls: (v: boolean) => void;
}

const Inp = ({ value, onChange, placeholder, type = 'text', mono = false }: {
  value: string; onChange: (v: string) => void; placeholder?: string; type?: string; mono?: boolean;
}) => (
  <input type={type} className="text-input" style={mono ? { fontFamily: 'var(--font-mono)', fontSize: '12px' } : undefined}
    value={value} onChange={(e) => onChange(e.target.value)} placeholder={placeholder} />
);

const Sel = ({ value, onChange, options }: {
  value: string; onChange: (v: string) => void; options: { value: string; label: string }[];
}) => (
  <select className="select-input" value={value} onChange={(e) => onChange(e.target.value)}>
    {options.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
  </select>
);

const Toggle = ({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) => (
  <label className="switch-toggle">
    <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
    <span className="switch-slider"></span>
  </label>
);

const SubCard = ({ title, children }: { title: string; children: React.ReactNode }) => (
  <div className="editor-subcard">
    <span className="editor-subcard-title">{title}</span>
    {children}
  </div>
);

export function NodeEditor(props: NodeEditorProps) {
  if (!props.isOpen) return null;

  const isSpecial = ['wireguard', 'ssh', 'socks', 'http'].includes(props.editProtocol);
  const noTransport = ['wireguard', 'ssh', 'socks', 'http', 'hysteria2', 'tuic'].includes(props.editProtocol);

  return (
    <>
      <div className="drawer-overlay" onClick={props.onClose} />
      <div className="drawer-panel">
        {/* Header */}
        <div className="drawer-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
            <span style={{ fontSize: '15px', fontWeight: 600, color: 'var(--text-high)' }}>Edit Node</span>
            <span className="type-chip" style={{ fontFamily: 'var(--font-mono)' }}>
              {PROTOCOL_LABELS[props.editProtocol] || props.editProtocol}
            </span>
          </div>
          <button className="btn-icon-only" onClick={props.onClose} style={{ border: 'none', background: 'transparent' }}>
            <X size={16} />
          </button>
        </div>

        {/* Section tabs */}
        {!isSpecial && (
          <div style={{ display: 'flex', gap: '2px', padding: '8px 20px 0', borderBottom: '1px solid var(--border-subtle)' }}>
            {(['basic', 'transport', 'tls'] as const).map((s) => (
              <button key={s} onClick={() => props.onSectionChange(s)}
                style={{
                  padding: '8px 14px', background: 'transparent', border: 'none', cursor: 'pointer',
                  fontSize: '12px', fontWeight: 500,
                  color: props.section === s ? 'var(--accent-primary)' : 'var(--text-med)',
                  borderBottom: props.section === s ? '2px solid var(--accent-primary)' : '2px solid transparent',
                  textTransform: 'capitalize', transition: 'all 0.2s',
                }}>
                {s === 'tls' ? 'TLS / Security' : s.charAt(0).toUpperCase() + s.slice(1)}
              </button>
            ))}
          </div>
        )}

        {/* Body */}
        <div className="drawer-body">
          {/* BASIC */}
          {(props.section === 'basic' || isSpecial) && (
            <>
              <SubCard title="Common">
                <div className="form-group">
                  <label className="form-label">Node Name (Tag) *</label>
                  <Inp value={props.editTag} onChange={props.setEditTag} placeholder="my-proxy-sg" />
                </div>
                <div className="form-group">
                  <label className="form-label">Protocol Type</label>
                  <Sel value={props.editProtocol} onChange={(v) => { props.setEditProtocol(v); props.onSectionChange('basic'); }} options={[
                    { value: 'vless', label: 'VLESS' }, { value: 'vmess', label: 'VMess' },
                    { value: 'trojan', label: 'Trojan' }, { value: 'shadowsocks', label: 'Shadowsocks' },
                    { value: 'hysteria2', label: 'Hysteria2' }, { value: 'tuic', label: 'TUIC v5' },
                    { value: 'wireguard', label: 'WireGuard' }, { value: 'ssh', label: 'SSH' },
                    { value: 'socks', label: 'SOCKS5' }, { value: 'http', label: 'HTTP Proxy' },
                  ]} />
                </div>
                {props.editProtocol !== 'wireguard' && (
                  <div className="editor-form-grid-2">
                    <div className="form-group">
                      <label className="form-label">Server Address *</label>
                      <Inp value={props.editAddress} onChange={props.setEditAddress} placeholder="sg.example.com" mono />
                    </div>
                    <div className="form-group">
                      <label className="form-label">Port *</label>
                      <input type="number" className="text-input" value={props.editPort} onChange={(e) => props.setEditPort(Number(e.target.value) || 443)} />
                    </div>
                  </div>
                )}
              </SubCard>

              {(props.editProtocol === 'vless' || props.editProtocol === 'vmess') && (
                <SubCard title="Authentication">
                  <div className="form-group">
                    <label className="form-label">UUID *</label>
                    <Inp value={props.editUuid} onChange={props.setEditUuid} placeholder="a20d4836-4634-471b-b6c3-2f48acb0e9a5" mono />
                  </div>
                  {props.editProtocol === 'vless' && (
                    <div className="form-group">
                      <label className="form-label">Flow (XTLS)</label>
                      <Sel value={props.editFlow} onChange={props.setEditFlow} options={[
                        { value: '', label: 'None' }, { value: 'xtls-rprx-vision', label: 'xtls-rprx-vision' },
                      ]} />
                    </div>
                  )}
                </SubCard>
              )}

              {props.editProtocol === 'trojan' && (
                <SubCard title="Trojan Authentication">
                  <div className="form-group">
                    <label className="form-label">Password *</label>
                    <input type="password" className="text-input" value={props.editPassword} onChange={(e) => props.setEditPassword(e.target.value)} placeholder="Trojan password" />
                  </div>
                </SubCard>
              )}

              {props.editProtocol === 'shadowsocks' && (
                <SubCard title="Shadowsocks">
                  <div className="form-group">
                    <label className="form-label">Password *</label>
                    <input type="password" className="text-input" value={props.editPassword} onChange={(e) => props.setEditPassword(e.target.value)} />
                  </div>
                  <div className="form-group">
                    <label className="form-label">Encryption Method *</label>
                    <Sel value={props.editMethod} onChange={props.setEditMethod} options={[
                      { value: 'aes-256-gcm', label: 'AES-256-GCM' },
                      { value: 'aes-128-gcm', label: 'AES-128-GCM' },
                      { value: 'chacha20-ietf-poly1305', label: 'ChaCha20-Poly1305' },
                      { value: '2022-blake3-aes-256-gcm', label: '2022-Blake3-AES-256-GCM' },
                    ]} />
                  </div>
                </SubCard>
              )}

              {props.editProtocol === 'hysteria2' && (
                <SubCard title="Hysteria2">
                  <div className="form-group">
                    <label className="form-label">Auth Password *</label>
                    <input type="password" className="text-input" value={props.editHy2Auth} onChange={(e) => props.setEditHy2Auth(e.target.value)} />
                  </div>
                  <div className="editor-form-grid-2">
                    <div className="form-group">
                      <label className="form-label">Upload BW</label>
                      <Inp value={props.editHy2UpBw} onChange={props.setEditHy2UpBw} placeholder="100 mbps" mono />
                    </div>
                    <div className="form-group">
                      <label className="form-label">Download BW</label>
                      <Inp value={props.editHy2DownBw} onChange={props.setEditHy2DownBw} placeholder="200 mbps" mono />
                    </div>
                  </div>
                  <div className="editor-form-grid-2">
                    <div className="form-group">
                      <label className="form-label">SNI</label>
                      <Inp value={props.editServerName} onChange={props.setEditServerName} placeholder="your-server.com" />
                    </div>
                    <div className="form-group">
                      <label className="form-label">Obfuscation</label>
                      <Sel value={props.editHy2ObfsType} onChange={props.setEditHy2ObfsType} options={[{ value: '', label: 'None' }, { value: 'salamander', label: 'Salamander' }]} />
                    </div>
                  </div>
                </SubCard>
              )}

              {props.editProtocol === 'tuic' && (
                <SubCard title="TUIC v5">
                  <div className="editor-form-grid-2">
                    <div className="form-group">
                      <label className="form-label">UUID *</label>
                      <Inp value={props.editTuicUuid} onChange={props.setEditTuicUuid} placeholder="UUID" mono />
                    </div>
                    <div className="form-group">
                      <label className="form-label">Password *</label>
                      <input type="password" className="text-input" value={props.editTuicPassword} onChange={(e) => props.setEditTuicPassword(e.target.value)} />
                    </div>
                  </div>
                  <div className="editor-form-grid-2">
                    <div className="form-group">
                      <label className="form-label">Congestion Control</label>
                      <Sel value={props.editTuicCongestion} onChange={props.setEditTuicCongestion} options={[
                        { value: 'bbr', label: 'BBR' }, { value: 'cubic', label: 'CUBIC' }, { value: 'new_reno', label: 'New Reno' },
                      ]} />
                    </div>
                    <div className="form-group">
                      <label className="form-label">UDP Relay Mode</label>
                      <Sel value={props.editTuicUdpMode} onChange={props.setEditTuicUdpMode} options={[
                        { value: 'native', label: 'Native' }, { value: 'quic', label: 'QUIC' },
                      ]} />
                    </div>
                  </div>
                </SubCard>
              )}

              {props.editProtocol === 'wireguard' && (
                <SubCard title="WireGuard">
                  <div className="form-group">
                    <label className="form-label">Client Private Key *</label>
                    <Inp value={props.editWgSecretKey} onChange={props.setEditWgSecretKey} placeholder="Base64 private key" mono />
                  </div>
                  <div className="form-group">
                    <label className="form-label">Server Endpoint (host:port) *</label>
                    <Inp value={props.editWgEndpoint} onChange={props.setEditWgEndpoint} placeholder="wg.example.com:51820" mono />
                  </div>
                  <div className="form-group">
                    <label className="form-label">Peer Public Key *</label>
                    <Inp value={props.editWgPeerPublicKey} onChange={props.setEditWgPeerPublicKey} placeholder="Base64 peer public key" mono />
                  </div>
                  <div className="form-group">
                    <label className="form-label">Allowed IPs</label>
                    <Inp value={props.editWgAllowedIps} onChange={props.setEditWgAllowedIps} placeholder="0.0.0.0/0, ::/0" mono />
                  </div>
                  <div className="editor-form-grid-2">
                    <div className="form-group">
                      <label className="form-label">Reserved (comma sep.)</label>
                      <Inp value={props.editWgReserved} onChange={props.setEditWgReserved} placeholder="0,0,0" mono />
                    </div>
                    <div className="form-group">
                      <label className="form-label">MTU</label>
                      <input type="number" className="text-input" value={props.editWgMtu} onChange={(e) => props.setEditWgMtu(parseInt(e.target.value) || 1280)} />
                    </div>
                  </div>
                </SubCard>
              )}

              {props.editProtocol === 'ssh' && (
                <SubCard title="SSH">
                  <div className="form-group">
                    <label className="form-label">Username *</label>
                    <Inp value={props.editSshUser} onChange={props.setEditSshUser} placeholder="root" mono />
                  </div>
                  <div className="form-group">
                    <label className="form-label">Password</label>
                    <input type="password" className="text-input" value={props.editSshPassword} onChange={(e) => props.setEditSshPassword(e.target.value)} placeholder="Leave empty for key auth" />
                  </div>
                  <div className="form-group">
                    <label className="form-label">Private Key (PEM)</label>
                    <textarea className="text-input" style={{ height: '70px', fontFamily: 'var(--font-mono)', fontSize: '11px', resize: 'vertical' }}
                      value={props.editSshPrivateKey} onChange={(e) => props.setEditSshPrivateKey(e.target.value)}
                      placeholder="-----BEGIN OPENSSH PRIVATE KEY-----" />
                  </div>
                </SubCard>
              )}

              {props.editProtocol === 'socks' && (
                <SubCard title="SOCKS">
                  <div className="editor-form-grid-2">
                    <div className="form-group">
                      <label className="form-label">SOCKS Version</label>
                      <Sel value={String(props.editSocksVersion)} onChange={(v) => props.setEditSocksVersion(parseInt(v))} options={[
                        { value: '5', label: 'SOCKS5' }, { value: '4', label: 'SOCKS4' },
                      ]} />
                    </div>
                    <div className="form-group">
                      <label className="form-label">Username</label>
                      <Inp value={props.editSocksUser} onChange={props.setEditSocksUser} placeholder="Optional" />
                    </div>
                  </div>
                  <div className="form-group">
                    <label className="form-label">Password</label>
                    <input type="password" className="text-input" value={props.editSocksPassword} onChange={(e) => props.setEditSocksPassword(e.target.value)} placeholder="Optional" />
                  </div>
                  <div className="switch-container" style={{ padding: '10px 14px' }}>
                    <div className="switch-details">
                      <span className="switch-title" style={{ fontSize: '13px' }}>UDP over TCP</span>
                    </div>
                    <Toggle checked={props.editSocksUdpOverTcp} onChange={props.setEditSocksUdpOverTcp} />
                  </div>
                </SubCard>
              )}

              {props.editProtocol === 'http' && (
                <SubCard title="HTTP Proxy">
                  <div className="editor-form-grid-2">
                    <div className="form-group">
                      <label className="form-label">Username</label>
                      <Inp value={props.editHttpUser} onChange={props.setEditHttpUser} placeholder="Optional" />
                    </div>
                    <div className="form-group">
                      <label className="form-label">Password</label>
                      <input type="password" className="text-input" value={props.editHttpPassword} onChange={(e) => props.setEditHttpPassword(e.target.value)} placeholder="Optional" />
                    </div>
                  </div>
                  <div className="switch-container" style={{ padding: '10px 14px' }}>
                    <div className="switch-details">
                      <span className="switch-title" style={{ fontSize: '13px' }}>HTTPS (TLS)</span>
                    </div>
                    <Toggle checked={props.editHttpTls} onChange={props.setEditHttpTls} />
                  </div>
                </SubCard>
              )}
            </>
          )}

          {/* TRANSPORT */}
          {props.section === 'transport' && !noTransport && (
            <SubCard title="Transport Layer">
              <div className="form-group">
                <label className="form-label">Network Transport</label>
                <Sel value={props.editNetwork} onChange={props.setEditNetwork} options={[
                  { value: 'tcp', label: 'TCP (Raw)' }, { value: 'ws', label: 'WebSocket' },
                  { value: 'grpc', label: 'gRPC' }, { value: 'http', label: 'HTTP/2' },
                  { value: 'httpupgrade', label: 'HTTP Upgrade' },
                ]} />
              </div>
              {(props.editNetwork === 'ws' || props.editNetwork === 'http' || props.editNetwork === 'httpupgrade') && (
                <div className="editor-form-grid-2">
                  <div className="form-group">
                    <label className="form-label">Path</label>
                    <Inp value={props.editPath} onChange={props.setEditPath} placeholder="/graphql" mono />
                  </div>
                  <div className="form-group">
                    <label className="form-label">Host Header</label>
                    <Inp value={props.editHost} onChange={props.setEditHost} placeholder="cdn.example.com" mono />
                  </div>
                </div>
              )}
              {props.editNetwork === 'grpc' && (
                <div className="form-group">
                  <label className="form-label">Service Name</label>
                  <Inp value={props.editServiceName} onChange={props.setEditServiceName} placeholder="GunService" mono />
                </div>
              )}
            </SubCard>
          )}

          {/* TLS */}
          {props.section === 'tls' && !noTransport && (
            <>
              <SubCard title="TLS Security">
                <div className="form-group">
                  <label className="form-label">Security Mode</label>
                  <Sel value={props.editTlsEnabled ? (props.editRealityEnabled ? 'reality' : 'tls') : 'none'}
                    onChange={(v) => {
                      if (v === 'none') { props.setEditTlsEnabled(false); props.setEditRealityEnabled(false); }
                      else if (v === 'tls') { props.setEditTlsEnabled(true); props.setEditRealityEnabled(false); }
                      else { props.setEditTlsEnabled(true); props.setEditRealityEnabled(true); }
                    }}
                    options={[{ value: 'none', label: 'None' }, { value: 'tls', label: 'TLS/SSL' }, { value: 'reality', label: 'REALITY (XTLS)' }]} />
                </div>
                {props.editTlsEnabled && (
                  <div className="form-group">
                    <label className="form-label">uTLS Fingerprint</label>
                    <Sel value={props.editFingerprint} onChange={props.setEditFingerprint} options={[
                      { value: 'chrome', label: 'Chrome' }, { value: 'firefox', label: 'Firefox' },
                      { value: 'edge', label: 'Edge' }, { value: 'safari', label: 'Safari' },
                      { value: 'random', label: 'Random' },
                    ]} />
                  </div>
                )}
              </SubCard>
              {props.editTlsEnabled && (
                <SubCard title="TLS Parameters">
                  <div className="editor-form-grid-2">
                    <div className="form-group">
                      <label className="form-label">SNI</label>
                      <Inp value={props.editServerName} onChange={props.setEditServerName} placeholder="aka.ms" mono />
                    </div>
                    <div className="form-group">
                      <label className="form-label">ALPN</label>
                      <Inp value={props.editAlpn} onChange={props.setEditAlpn} placeholder="h2, http/1.1" mono />
                    </div>
                  </div>
                  <div className="editor-checkbox-row" style={{ marginTop: '8px' }}>
                    <input type="checkbox" id="nodeAllowInsecure" checked={props.editAllowInsecure} onChange={(e) => props.setEditAllowInsecure(e.target.checked)} />
                    <label htmlFor="nodeAllowInsecure" className="editor-checkbox-label">Allow Insecure (skip cert verification)</label>
                  </div>
                </SubCard>
              )}
              {props.editTlsEnabled && props.editRealityEnabled && (
                <SubCard title="REALITY Configuration">
                  <div className="info-box" style={{ marginBottom: '12px' }}>
                    <Info size={12} />
                    <span>REALITY impersonates a real TLS server to bypass censorship.</span>
                  </div>
                  <div className="form-group">
                    <label className="form-label">Public Key (pbk) *</label>
                    <Inp value={props.editPublicKey} onChange={props.setEditPublicKey} placeholder="Base64 public key..." mono />
                  </div>
                  <div className="editor-form-grid-2">
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">Short ID (sid)</label>
                      <Inp value={props.editShortId} onChange={props.setEditShortId} placeholder="97c4e5fcb1e8" mono />
                    </div>
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">Destination SNI</label>
                      <Inp value={props.editServerName} onChange={props.setEditServerName} placeholder="aka.ms" mono />
                    </div>
                  </div>
                </SubCard>
              )}
            </>
          )}

          {props.saveError && (
            <div className="alert-box error" style={{ marginTop: '8px' }}>
              <ShieldAlert size={14} /><span>{props.saveError}</span>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="drawer-footer">
          <button className="btn secondary sm" onClick={props.onApplyPreset} style={{ marginRight: 'auto' }}>
            Preset
          </button>
          <button className="btn secondary" onClick={props.onClose}>Cancel</button>
          <button className="btn primary" onClick={props.onSave} disabled={props.isSaving} style={{ minWidth: '90px' }}>
            {props.isSaving ? <><RefreshCw size={14} className="spin" /> Saving…</> : 'Save Node'}
          </button>
        </div>
      </div>
    </>
  );
}
