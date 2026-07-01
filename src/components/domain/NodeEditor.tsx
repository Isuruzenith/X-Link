import React from 'react';
import { X, RefreshCw, ShieldAlert, Info } from 'lucide-react';
import { useNodeEditorStore } from '../../stores/nodeEditorStore';

const PROTOCOL_LABELS: Record<string, string> = {
  vless: 'VLESS',
  vmess: 'VMess',
  trojan: 'Trojan',
  shadowsocks: 'Shadowsocks',
  hysteria2: 'Hysteria2',
  tuic: 'TUIC v5',
  socks: 'SOCKS5',
  http: 'HTTP',
};

const Inp = ({ value, onChange, placeholder, type = 'text', mono = false }: {
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  type?: string;
  mono?: boolean;
}) => (
  <input
    type={type}
    className="text-input"
    style={mono ? { fontFamily: 'var(--font-mono)', fontSize: '12px' } : undefined}
    value={value}
    onChange={(e) => onChange(e.target.value)}
    placeholder={placeholder}
  />
);

const Sel = ({ value, onChange, options }: {
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
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

export function NodeEditor() {
  const store = useNodeEditorStore();

  if (!store.isOpen) return null;

  const isSpecial = ['socks', 'http'].includes(store.editProtocol);
  const noTransport = ['socks', 'http', 'hysteria2', 'tuic'].includes(store.editProtocol);

  return (
    <>
      <div className="drawer-overlay" onClick={store.closeEditor} />
      <div className="drawer-panel">
        {/* Header */}
        <div className="drawer-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
            <span style={{ fontSize: '15px', fontWeight: 600, color: 'var(--text-high)' }}>Edit Node</span>
            <span className="type-chip" style={{ fontFamily: 'var(--font-mono)' }}>
              {PROTOCOL_LABELS[store.editProtocol] || store.editProtocol}
            </span>
          </div>
          <button className="btn-icon-only" onClick={store.closeEditor} style={{ border: 'none', background: 'transparent' }}>
            <X size={16} />
          </button>
        </div>

        {/* Section tabs */}
        {!isSpecial && (
          <div style={{ display: 'flex', gap: '2px', padding: '8px 20px 0', borderBottom: '1px solid var(--border-subtle)' }}>
            {(['basic', 'transport', 'tls'] as const).map((s) => (
              <button
                key={s}
                onClick={() => store.setSection(s)}
                style={{
                  padding: '8px 14px',
                  background: 'transparent',
                  border: 'none',
                  cursor: 'pointer',
                  fontSize: '12px',
                  fontWeight: 500,
                  color: store.section === s ? 'var(--accent-primary)' : 'var(--text-med)',
                  borderBottom: store.section === s ? '2px solid var(--accent-primary)' : '2px solid transparent',
                  textTransform: 'capitalize',
                  transition: 'all 0.2s',
                }}
              >
                {s === 'tls' ? 'TLS / Security' : s.charAt(0).toUpperCase() + s.slice(1)}
              </button>
            ))}
          </div>
        )}

        {/* Body */}
        <div className="drawer-body">
          {/* BASIC */}
          {(store.section === 'basic' || isSpecial) && (
            <>
              <SubCard title="Common">
                <div className="form-group">
                  <label className="form-label">Node Name (Tag) *</label>
                  <Inp value={store.editTag} onChange={(v) => store.setField('editTag', v)} placeholder="my-proxy-sg" />
                </div>
                <div className="form-group">
                  <label className="form-label">Protocol Type</label>
                  <Sel
                    value={store.editProtocol}
                    onChange={(v) => {
                      store.setField('editProtocol', v);
                      store.setSection('basic');
                    }}
                    options={[
                      { value: 'vless', label: 'VLESS' },
                      { value: 'vmess', label: 'VMess' },
                      { value: 'trojan', label: 'Trojan' },
                      { value: 'shadowsocks', label: 'Shadowsocks' },
                      { value: 'hysteria2', label: 'Hysteria2' },
                      { value: 'tuic', label: 'TUIC v5' },
                      { value: 'socks', label: 'SOCKS5' },
                      { value: 'http', label: 'HTTP Proxy' },
                    ]}
                  />
                </div>
                <div className="editor-form-grid-2">
                  <div className="form-group">
                    <label className="form-label">Server Address *</label>
                    <Inp value={store.editAddress} onChange={(v) => store.setField('editAddress', v)} placeholder="sg.example.com" mono />
                  </div>
                  <div className="form-group">
                    <label className="form-label">Port *</label>
                    <input
                      type="number"
                      className="text-input"
                      value={store.editPort}
                      onChange={(e) => store.setField('editPort', Number(e.target.value) || 443)}
                    />
                  </div>
                </div>
              </SubCard>

              {(store.editProtocol === 'vless' || store.editProtocol === 'vmess') && (
                <SubCard title="Authentication">
                  <div className="form-group">
                    <label className="form-label">UUID *</label>
                    <Inp value={store.editUuid} onChange={(v) => store.setField('editUuid', v)} placeholder="uuid-here" mono />
                  </div>
                  {store.editProtocol === 'vless' && (
                    <div className="form-group">
                      <label className="form-label">Flow (XTLS)</label>
                      <Sel
                        value={store.editFlow}
                        onChange={(v) => store.setField('editFlow', v)}
                        options={[
                          { value: '', label: 'None' },
                          { value: 'xtls-rprx-vision', label: 'xtls-rprx-vision' },
                        ]}
                      />
                    </div>
                  )}
                </SubCard>
              )}

              {store.editProtocol === 'trojan' && (
                <SubCard title="Trojan Authentication">
                  <div className="form-group">
                    <label className="form-label">Password *</label>
                    <input
                      type="password"
                      className="text-input"
                      value={store.editPassword}
                      onChange={(e) => store.setField('editPassword', e.target.value)}
                      placeholder="Trojan password"
                    />
                  </div>
                </SubCard>
              )}

              {store.editProtocol === 'shadowsocks' && (
                <SubCard title="Shadowsocks">
                  <div className="form-group">
                    <label className="form-label">Password *</label>
                    <input
                      type="password"
                      className="text-input"
                      value={store.editPassword}
                      onChange={(e) => store.setField('editPassword', e.target.value)}
                    />
                  </div>
                  <div className="form-group">
                    <label className="form-label">Encryption Method *</label>
                    <Sel
                      value={store.editMethod}
                      onChange={(v) => store.setField('editMethod', v)}
                      options={[
                        { value: 'aes-256-gcm', label: 'AES-256-GCM' },
                        { value: 'aes-128-gcm', label: 'AES-128-GCM' },
                        { value: 'chacha20-ietf-poly1305', label: 'ChaCha20-Poly1305' },
                        { value: '2022-blake3-aes-256-gcm', label: '2022-Blake3-AES-256-GCM' },
                      ]}
                    />
                  </div>
                </SubCard>
              )}

              {store.editProtocol === 'hysteria2' && (
                <SubCard title="Hysteria2">
                  <div className="form-group">
                    <label className="form-label">Auth Password *</label>
                    <input
                      type="password"
                      className="text-input"
                      value={store.editHy2Auth}
                      onChange={(e) => store.setField('editHy2Auth', e.target.value)}
                    />
                  </div>
                  <div className="editor-form-grid-2">
                    <div className="form-group">
                      <label className="form-label">Upload BW</label>
                      <Inp value={store.editHy2UpBw} onChange={(v) => store.setField('editHy2UpBw', v)} placeholder="100 mbps" mono />
                    </div>
                    <div className="form-group">
                      <label className="form-label">Download BW</label>
                      <Inp value={store.editHy2DownBw} onChange={(v) => store.setField('editHy2DownBw', v)} placeholder="200 mbps" mono />
                    </div>
                  </div>
                  <div className="editor-form-grid-2">
                    <div className="form-group">
                      <label className="form-label">SNI</label>
                      <Inp value={store.editServerName} onChange={(v) => store.setField('editServerName', v)} placeholder="your-server.com" />
                    </div>
                    <div className="form-group">
                      <label className="form-label">Obfuscation</label>
                      <Sel
                        value={store.editHy2ObfsType}
                        onChange={(v) => store.setField('editHy2ObfsType', v)}
                        options={[
                          { value: '', label: 'None' },
                          { value: 'salamander', label: 'Salamander' },
                        ]}
                      />
                    </div>
                  </div>
                </SubCard>
              )}

              {store.editProtocol === 'tuic' && (
                <SubCard title="TUIC v5">
                  <div className="editor-form-grid-2">
                    <div className="form-group">
                      <label className="form-label">UUID *</label>
                      <Inp value={store.editTuicUuid} onChange={(v) => store.setField('editTuicUuid', v)} placeholder="UUID" mono />
                    </div>
                    <div className="form-group">
                      <label className="form-label">Password *</label>
                      <input
                        type="password"
                        className="text-input"
                        value={store.editTuicPassword}
                        onChange={(e) => store.setField('editTuicPassword', e.target.value)}
                      />
                    </div>
                  </div>
                  <div className="editor-form-grid-2">
                    <div className="form-group">
                      <label className="form-label">Congestion Control</label>
                      <Sel
                        value={store.editTuicCongestion}
                        onChange={(v) => store.setField('editTuicCongestion', v)}
                        options={[
                          { value: 'bbr', label: 'BBR' },
                          { value: 'cubic', label: 'CUBIC' },
                          { value: 'new_reno', label: 'New Reno' },
                        ]}
                      />
                    </div>
                    <div className="form-group">
                      <label className="form-label">UDP Relay Mode</label>
                      <Sel
                        value={store.editTuicUdpMode}
                        onChange={(v) => store.setField('editTuicUdpMode', v)}
                        options={[
                          { value: 'native', label: 'Native' },
                          { value: 'quic', label: 'QUIC' },
                        ]}
                      />
                    </div>
                  </div>
                </SubCard>
              )}

              {store.editProtocol === 'socks' && (
                <SubCard title="SOCKS">
                  <div className="editor-form-grid-2">
                    <div className="form-group">
                      <label className="form-label">SOCKS Version</label>
                      <Sel
                        value={String(store.editSocksVersion)}
                        onChange={(v) => store.setField('editSocksVersion', parseInt(v))}
                        options={[
                          { value: '5', label: 'SOCKS5' },
                          { value: '4', label: 'SOCKS4' },
                        ]}
                      />
                    </div>
                    <div className="form-group">
                      <label className="form-label">Username</label>
                      <Inp value={store.editSocksUser} onChange={(v) => store.setField('editSocksUser', v)} placeholder="Optional" />
                    </div>
                  </div>
                  <div className="form-group">
                    <label className="form-label">Password</label>
                    <input
                      type="password"
                      className="text-input"
                      value={store.editSocksPassword}
                      onChange={(e) => store.setField('editSocksPassword', e.target.value)}
                      placeholder="Optional"
                    />
                  </div>
                  <div className="switch-container" style={{ padding: '10px 14px' }}>
                    <div className="switch-details">
                      <span className="switch-title" style={{ fontSize: '13px' }}>UDP over TCP</span>
                    </div>
                    <Toggle checked={store.editSocksUdpOverTcp} onChange={(v) => store.setField('editSocksUdpOverTcp', v)} />
                  </div>
                </SubCard>
              )}

              {store.editProtocol === 'http' && (
                <SubCard title="HTTP Proxy">
                  <div className="editor-form-grid-2">
                    <div className="form-group">
                      <label className="form-label">Username</label>
                      <Inp value={store.editHttpUser} onChange={(v) => store.setField('editHttpUser', v)} placeholder="Optional" />
                    </div>
                    <div className="form-group">
                      <label className="form-label">Password</label>
                      <input
                        type="password"
                        className="text-input"
                        value={store.editHttpPassword}
                        onChange={(e) => store.setField('editHttpPassword', e.target.value)}
                        placeholder="Optional"
                      />
                    </div>
                  </div>
                  <div className="switch-container" style={{ padding: '10px 14px' }}>
                    <div className="switch-details">
                      <span className="switch-title" style={{ fontSize: '13px' }}>HTTPS (TLS)</span>
                    </div>
                    <Toggle checked={store.editHttpTls} onChange={(v) => store.setField('editHttpTls', v)} />
                  </div>
                </SubCard>
              )}
            </>
          )}

          {/* TRANSPORT */}
          {store.section === 'transport' && !noTransport && (
            <SubCard title="Transport Layer">
              <div className="form-group">
                <label className="form-label">Network Transport</label>
                <Sel
                  value={store.editNetwork}
                  onChange={(v) => store.setField('editNetwork', v)}
                  options={[
                    { value: 'tcp', label: 'TCP (Raw)' },
                    { value: 'ws', label: 'WebSocket' },
                    { value: 'grpc', label: 'gRPC' },
                    { value: 'http', label: 'HTTP/2' },
                    { value: 'httpupgrade', label: 'HTTP Upgrade' },
                  ]}
                />
              </div>
              {(store.editNetwork === 'ws' || store.editNetwork === 'http' || store.editNetwork === 'httpupgrade') && (
                <div className="editor-form-grid-2">
                  <div className="form-group">
                    <label className="form-label">Path</label>
                    <Inp value={store.editPath} onChange={(v) => store.setField('editPath', v)} placeholder="/graphql" mono />
                  </div>
                  <div className="form-group">
                    <label className="form-label">Host Header</label>
                    <Inp value={store.editHost} onChange={(v) => store.setField('editHost', v)} placeholder="cdn.example.com" mono />
                  </div>
                </div>
              )}
              {store.editNetwork === 'grpc' && (
                <div className="form-group">
                  <label className="form-label">Service Name</label>
                  <Inp value={store.editServiceName} onChange={(v) => store.setField('editServiceName', v)} placeholder="GunService" mono />
                </div>
              )}
            </SubCard>
          )}

          {/* TLS */}
          {store.section === 'tls' && !noTransport && (
            <>
              <SubCard title="TLS Security">
                <div className="form-group">
                  <label className="form-label">Security Mode</label>
                  <Sel
                    value={store.editTlsEnabled ? (store.editRealityEnabled ? 'reality' : 'tls') : 'none'}
                    onChange={(v) => {
                      if (v === 'none') {
                        store.setField('editTlsEnabled', false);
                        store.setField('editRealityEnabled', false);
                      } else if (v === 'tls') {
                        store.setField('editTlsEnabled', true);
                        store.setField('editRealityEnabled', false);
                      } else {
                        store.setField('editTlsEnabled', true);
                        store.setField('editRealityEnabled', true);
                      }
                    }}
                    options={[
                      { value: 'none', label: 'None' },
                      { value: 'tls', label: 'TLS/SSL' },
                      { value: 'reality', label: 'REALITY (XTLS)' },
                    ]}
                  />
                </div>
                {store.editTlsEnabled && (
                  <div className="form-group">
                    <label className="form-label">uTLS Fingerprint</label>
                    <Sel
                      value={store.editFingerprint}
                      onChange={(v) => store.setField('editFingerprint', v)}
                      options={[
                        { value: 'chrome', label: 'Chrome' },
                        { value: 'firefox', label: 'Firefox' },
                        { value: 'edge', label: 'Edge' },
                        { value: 'safari', label: 'Safari' },
                        { value: 'random', label: 'Random' },
                      ]}
                    />
                  </div>
                )}
              </SubCard>
              {store.editTlsEnabled && (
                <SubCard title="TLS Parameters">
                  <div className="editor-form-grid-2">
                    <div className="form-group">
                      <label className="form-label">SNI</label>
                      <Inp value={store.editServerName} onChange={(v) => store.setField('editServerName', v)} placeholder="aka.ms" mono />
                    </div>
                    <div className="form-group">
                      <label className="form-label">ALPN</label>
                      <Inp value={store.editAlpn} onChange={(v) => store.setField('editAlpn', v)} placeholder="h2, http/1.1" mono />
                    </div>
                  </div>
                  <div className="editor-checkbox-row" style={{ marginTop: '8px' }}>
                    <input
                      type="checkbox"
                      id="nodeAllowInsecure"
                      checked={store.editAllowInsecure}
                      onChange={(e) => store.setField('editAllowInsecure', e.target.checked)}
                    />
                    <label htmlFor="nodeAllowInsecure" className="editor-checkbox-label">
                      Allow Insecure (skip cert verification)
                    </label>
                  </div>
                </SubCard>
              )}
              {store.editTlsEnabled && store.editRealityEnabled && (
                <SubCard title="REALITY Configuration">
                  <div className="info-box" style={{ marginBottom: '12px' }}>
                    <Info size={12} />
                    <span>REALITY impersonates a real TLS server to bypass censorship.</span>
                  </div>
                  <div className="form-group">
                    <label className="form-label">Public Key (pbk) *</label>
                    <Inp value={store.editPublicKey} onChange={(v) => store.setField('editPublicKey', v)} placeholder="Base64 public key..." mono />
                  </div>
                  <div className="editor-form-grid-2">
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">Short ID (sid)</label>
                      <Inp value={store.editShortId} onChange={(v) => store.setField('editShortId', v)} placeholder="97c4e5fcb1e8" mono />
                    </div>
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">Destination SNI</label>
                      <Inp value={store.editServerName} onChange={(v) => store.setField('editServerName', v)} placeholder="aka.ms" mono />
                    </div>
                  </div>
                </SubCard>
              )}
            </>
          )}

          {store.saveError && (
            <div className="alert-box error" style={{ marginTop: '8px' }}>
              <ShieldAlert size={14} />
              <span>{store.saveError}</span>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="drawer-footer">
          <button className="btn secondary" onClick={store.closeEditor}>Cancel</button>
          <button className="btn primary" onClick={store.saveEditor} disabled={store.isSaving} style={{ minWidth: '90px' }}>
            {store.isSaving ? <><RefreshCw size={14} className="spin" /> Saving…</> : 'Save Node'}
          </button>
        </div>
      </div>
    </>
  );
}
