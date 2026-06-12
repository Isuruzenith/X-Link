import React from 'react';
import {
  Settings as SettingsIcon, Network, Eye, Layers2, Code2,
  Info, AlertTriangle, CheckCircle2, ExternalLink
} from 'lucide-react';
import { ViewShell } from '../components/ViewShell';
import type { Settings } from '../utils/store';
import xLinkLogo from '../assets/X-Link-logo.png';

const Inp = ({ value, onChange, placeholder, mono = false }: {
  value: string; onChange: (v: string) => void; placeholder?: string; mono?: boolean;
}) => (
  <input className="text-input" style={mono ? { fontFamily: 'var(--font-mono)', fontSize: '12px' } : undefined}
    value={value} onChange={(e) => onChange(e.target.value)} placeholder={placeholder} />
);

const Sel = ({ value, onChange, options }: {
  value: string; onChange: (v: string) => void; options: { value: string; label: string }[];
}) => (
  <select className="select-input" value={value} onChange={(e) => onChange(e.target.value)}>
    {options.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
  </select>
);

const Toggle = ({ checked, onChange, disabled }: { checked: boolean; onChange: (v: boolean) => void; disabled?: boolean }) => (
  <label className="switch-toggle" style={disabled ? { opacity: 0.4, pointerEvents: 'none' } : undefined}>
    <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} disabled={disabled} />
    <span className="switch-slider"></span>
  </label>
);

const SwitchRow = ({ title, desc, checked, onChange, disabled }: {
  title: string; desc?: string; checked: boolean; onChange: (v: boolean) => void; disabled?: boolean;
}) => (
  <div className="switch-container">
    <div className="switch-details">
      <span className="switch-title">{title}</span>
      {desc && <span className="switch-desc">{desc}</span>}
    </div>
    <Toggle checked={checked} onChange={onChange} disabled={disabled} />
  </div>
);

type SettingsSection = 'general' | 'tun' | 'sniff' | 'mux' | 'api';

interface SettingsViewProps {
  settings: Settings;
  settingsTab: SettingsSection;
  conflictingPorts: number[];
  isElevated: boolean;
  singboxVersion: string;
  appVersion: string;
  httpPort: number;
  socksPort: number;
  mixedPort: number;
  isConnected: boolean;
  onSetSettingsTab: (t: SettingsSection) => void;
  onSaveSettings: (updates: Partial<Settings>) => void;
  onSetHttpPort: (v: number) => void;
  onSetSocksPort: (v: number) => void;
  onSetMixedPort: (v: number) => void;
}

export function SettingsView({
  settings, settingsTab, conflictingPorts, isElevated, appVersion,
  httpPort, socksPort, mixedPort, isConnected,
  onSetSettingsTab, onSaveSettings, onSetHttpPort, onSetSocksPort, onSetMixedPort,
}: SettingsViewProps) {
  const sideNavItems: [SettingsSection, React.ComponentType<any>, string][] = [
    ['general', SettingsIcon, 'General'],
    ['tun',     Network,      'TUN Mode'],
    ['sniff',   Eye,          'Sniffing'],
    ['mux',     Layers2,      'Multiplexing'],
    ['api',     Code2,        'API'],
  ];

  return (
    <ViewShell title="Settings" subtitle="Proxy mode, TUN, sniffing, multiplexing, API and performance options">
      <div className="settings-layout" style={{ height: 'calc(100vh - 100px)' }}>
        {/* Secondary nav */}
        <div className="settings-sidenav">
          {sideNavItems.map(([id, Icon, label]) => (
            <div key={id} className={`settings-nav-item ${settingsTab === id ? 'active' : ''}`} onClick={() => onSetSettingsTab(id)}>
              <Icon size={14} />{label}
            </div>
          ))}
        </div>

        {/* Content */}
        <div className="settings-content">

          {/* ── GENERAL ── */}
          {settingsTab === 'general' && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
              <div className="grid-2">
                <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                  <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                    <div className="settings-section-heading">Proxy Mode</div>
                    <SwitchRow title="Proxy Mode: TUN vs System Proxy" desc="Toggle between native routing (TUN) and OS settings (System Proxy). TUN mode captures all system traffic but requires Administrator."
                      checked={settings.proxyMode === 'tun'} onChange={(v) => onSaveSettings({ proxyMode: v ? 'tun' : 'system' })} />
                  </div>

                  <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                    <div className="settings-section-heading">App Preferences</div>
                    <SwitchRow title="Close to Tray" desc="Minimize to system tray on close" checked={settings.closeToTray} onChange={(v) => onSaveSettings({ closeToTray: v })} />
                    <SwitchRow title="Autostart with Windows" desc="Launch minimized on user login" checked={settings.autostart} onChange={(v) => onSaveSettings({ autostart: v })} />
                    <SwitchRow title="LAN Hotspot Sharing" desc="Bind to 0.0.0.0 for LAN device sharing" checked={settings.wifiSharing} onChange={(v) => onSaveSettings({ wifiSharing: v })} />
                  </div>
                </div>

                <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                  <div className="settings-section-heading">Inbound Ports</div>
                  <div className="grid-2">
                    {[
                      { label: 'HTTP Proxy Port', key: 'httpPort', val: httpPort, set: onSetHttpPort },
                      { label: 'SOCKS5 Port', key: 'socksPort', val: socksPort, set: onSetSocksPort },
                    ].map(({ label, key, val, set }) => (
                      <div key={key} className="form-group">
                        <div className="flex-row-between">
                          <label className="form-label">{label}</label>
                          <span style={{ fontSize: '10px', fontWeight: 600, color: conflictingPorts.includes(val) ? 'var(--status-err)' : 'var(--status-ok)' }}>
                            {conflictingPorts.includes(val) ? '● Conflict' : '● OK'}
                          </span>
                        </div>
                        <input type="number" className="text-input" style={{ borderColor: conflictingPorts.includes(val) ? 'var(--status-err)' : undefined }}
                          value={val} onChange={(e) => { const v = parseInt(e.target.value) || 0; set(v); onSaveSettings({ [key]: v } as any); }} />
                      </div>
                    ))}
                  </div>
                  <div className="form-group" style={{ marginBottom: 0 }}>
                    <div className="flex-row-between">
                      <label className="form-label">Mixed Inbound Port (Recommended)</label>
                      <span style={{ fontSize: '10px', fontWeight: 600, color: conflictingPorts.includes(mixedPort) ? 'var(--status-err)' : 'var(--status-ok)' }}>
                        {conflictingPorts.includes(mixedPort) ? '● Conflict' : '● OK'}
                      </span>
                    </div>
                    <input type="number" className="text-input" style={{ borderColor: conflictingPorts.includes(mixedPort) ? 'var(--status-err)' : undefined }}
                      value={mixedPort} onChange={(e) => { const v = parseInt(e.target.value) || 0; onSetMixedPort(v); onSaveSettings({ mixedPort: v }); }} />
                  </div>
                  <p style={{ fontSize: '11px', color: 'var(--text-low)' }}>Port changes take effect on next connection restart.</p>
                </div>
              </div>

              <div className="glass-panel" style={{ display: 'flex', alignItems: 'center', gap: '16px', padding: '16px 20px' }}>
                <img src={xLinkLogo} alt="X-Link" style={{ width: '40px', height: '40px', borderRadius: '8px', flexShrink: 0 }} />
                <div>
                  <h4 style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-high)' }}>X-Link</h4>
                  <p style={{ fontSize: '11px', color: 'var(--text-low)', marginTop: '1px' }}>
                    Empowered by sing-box
                  </p>
                </div>
                <div style={{ marginLeft: 'auto' }}>
                  <span className="chip success" style={{ textTransform: 'none', fontSize: '10px', padding: '2px 8px' }}>v{appVersion}</span>
                </div>
              </div>
            </div>
          )}

          {/* ── TUN MODE ── */}
          {settingsTab === 'tun' && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
              {!isElevated && (
                <div className="alert-box warn">
                  <AlertTriangle size={14} />
                  <span>TUN Mode requires Administrator elevation. Click "Run as Admin" on the Dashboard.</span>
                </div>
              )}
              <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                <div className="settings-section-heading">TUN Interface Configuration</div>
                <div className="grid-2">
                  <div className="form-group" style={{ marginBottom: 0 }}>
                    <label className="form-label">TUN Stack Implementation</label>
                    <Sel value={settings.tunStack} onChange={(v) => onSaveSettings({ tunStack: v as any })} options={[
                      { value: 'mixed', label: 'Mixed (gVisor TCP + system UDP)' },
                      { value: 'gvisor', label: 'gVisor (Full userspace)' },
                      { value: 'system', label: 'System (Kernel-level)' },
                    ]} />
                    <span style={{ fontSize: '11px', color: 'var(--text-low)', marginTop: '4px', display: 'block' }}>Mixed: best compatibility. gVisor: best isolation.</span>
                  </div>
                  <div className="form-group" style={{ marginBottom: 0 }}>
                    <label className="form-label">TUN MTU</label>
                    <input type="number" className="text-input" value={settings.tunMtu} onChange={(e) => onSaveSettings({ tunMtu: parseInt(e.target.value) || 9000 })} />
                    <span style={{ fontSize: '11px', color: 'var(--text-low)', marginTop: '4px', display: 'block' }}>9000 recommended. Use 1500 for strict environments.</span>
                  </div>
                </div>
                {[
                  { key: 'tunAutoRoute', title: 'Auto Route', desc: 'Add routes to direct traffic into TUN interface' },
                  { key: 'tunAutoRedirect', title: 'Auto Redirect (nftables)', desc: 'Use nftables-based redirect on Linux' },
                  { key: 'tunStrictRoute', title: 'Strict Route', desc: 'Block unmatched traffic to prevent leaks' },
                  { key: 'tunEndpointIndependentNat', title: 'Endpoint Independent NAT', desc: 'Improves UDP NAT traversal (P2P, gaming)' },
                ].map(({ key, title, desc }) => (
                  <SwitchRow key={key} title={title} desc={desc} checked={(settings as any)[key]} onChange={(v) => onSaveSettings({ [key]: v } as any)} disabled={!isElevated} />
                ))}
              </div>
              <div className="info-box">
                <Info size={13} />
                <span>TUN mode captures ALL system traffic. Combined with Auto Route + Strict Route, it prevents DNS leaks.</span>
              </div>
            </div>
          )}

          {/* ── SNIFFING ── */}
          {settingsTab === 'sniff' && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
              <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                <div className="settings-section-heading">Traffic Sniffing & Protocol Detection</div>
                <SwitchRow title="Enable Traffic Sniffing" desc="Inspect connection metadata to extract domain names for routing"
                  checked={settings.sniffEnabled} onChange={(v) => onSaveSettings({ sniffEnabled: v })} />
                <div style={{ opacity: settings.sniffEnabled ? 1 : 0.4, pointerEvents: settings.sniffEnabled ? 'auto' : 'none' }}>
                  <div className="grid-2">
                    {[
                      { key: 'sniffHttp', title: 'HTTP Sniffing', desc: 'Extract hostname from HTTP Host header' },
                      { key: 'sniffTls', title: 'TLS/HTTPS Sniffing', desc: 'Extract SNI from TLS ClientHello' },
                      { key: 'sniffQuic', title: 'QUIC/HTTP3 Sniffing', desc: 'Extract SNI from QUIC Initial packets' },
                      { key: 'sniffOverrideDestination', title: 'Override Destination', desc: 'Use sniffed domain to override target IP' },
                    ].map(({ key, title, desc }) => (
                      <SwitchRow key={key} title={title} desc={desc} checked={(settings as any)[key]} onChange={(v) => onSaveSettings({ [key]: v } as any)} />
                    ))}
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* ── MULTIPLEXING ── */}
          {settingsTab === 'mux' && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
              <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                <div className="settings-section-heading">Connection Multiplexing</div>
                <SwitchRow title="Enable Multiplexing (Mux)" desc="Bundle multiple streams into single connection to reduce TLS overhead"
                  checked={settings.muxEnabled} onChange={(v) => onSaveSettings({ muxEnabled: v })} />
                <div style={{ opacity: settings.muxEnabled ? 1 : 0.4, pointerEvents: settings.muxEnabled ? 'auto' : 'none', display: 'flex', flexDirection: 'column', gap: '16px' }}>
                  <div className="grid-2">
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">Mux Protocol</label>
                      <Sel value={settings.muxProtocol} onChange={(v) => onSaveSettings({ muxProtocol: v as any })} options={[
                        { value: 'h2mux', label: 'h2mux (HTTP/2, recommended)' },
                        { value: 'smux', label: 'smux (Simple stream mux)' },
                        { value: 'yamux', label: 'yamux (HashiCorp Yamux)' },
                      ]} />
                    </div>
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">Max Connections</label>
                      <input type="number" className="text-input" value={settings.muxMaxConnections} onChange={(e) => onSaveSettings({ muxMaxConnections: parseInt(e.target.value) || 4 })} />
                    </div>
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">Min Streams</label>
                      <input type="number" className="text-input" value={settings.muxMinStreams} onChange={(e) => onSaveSettings({ muxMinStreams: parseInt(e.target.value) || 4 })} />
                    </div>
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">Max Streams (0 = unlimited)</label>
                      <input type="number" className="text-input" value={settings.muxMaxStreams} onChange={(e) => onSaveSettings({ muxMaxStreams: parseInt(e.target.value) || 0 })} />
                    </div>
                  </div>
                  <div className="grid-2">
                    <SwitchRow title="Stream Padding" desc="Add random padding to obfuscate stream lengths" checked={settings.muxPadding} onChange={(v) => onSaveSettings({ muxPadding: v })} />
                    <SwitchRow title="Brutal Congestion Control" desc="Fixed bandwidth mode instead of BBR" checked={settings.muxBrutal} onChange={(v) => onSaveSettings({ muxBrutal: v })} />
                  </div>
                  {settings.muxBrutal && (
                    <div className="grid-2">
                      <div className="form-group" style={{ marginBottom: 0 }}>
                        <label className="form-label">Brutal Upload Speed (Mbps)</label>
                        <input type="number" className="text-input" value={settings.muxBrutalUpMbps} onChange={(e) => onSaveSettings({ muxBrutalUpMbps: parseFloat(e.target.value) || 100 })} />
                      </div>
                      <div className="form-group" style={{ marginBottom: 0 }}>
                        <label className="form-label">Brutal Download Speed (Mbps)</label>
                        <input type="number" className="text-input" value={settings.muxBrutalDownMbps} onChange={(e) => onSaveSettings({ muxBrutalDownMbps: parseFloat(e.target.value) || 100 })} />
                      </div>
                    </div>
                  )}
                </div>
              </div>
            </div>
          )}

          {/* ── API ── */}
          {settingsTab === 'api' && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
              <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                <div className="settings-section-heading">Clash-Compatible REST API</div>
                <SwitchRow title="Enable External API" desc="Expose HTTP API for external controllers (Yacd, MetaCubeX)"
                  checked={settings.apiEnabled} onChange={(v) => onSaveSettings({ apiEnabled: v })} />
                <div style={{ opacity: settings.apiEnabled ? 1 : 0.4, pointerEvents: settings.apiEnabled ? 'auto' : 'none', display: 'flex', flexDirection: 'column', gap: '16px' }}>
                  <div className="grid-2">
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">API Port</label>
                      <input type="number" className="text-input" value={settings.apiPort} onChange={(e) => onSaveSettings({ apiPort: parseInt(e.target.value) || 9090 })} />
                    </div>
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">API Secret Token</label>
                      <Inp value={settings.apiSecret} onChange={(v) => onSaveSettings({ apiSecret: v })} placeholder="Leave empty for no auth" mono />
                    </div>
                  </div>
                  <SwitchRow title="Allow CORS (Cross-Origin)" desc="Allow web-based controllers (Yacd) to connect from browser"
                    checked={settings.apiCors} onChange={(v) => onSaveSettings({ apiCors: v })} />
                  {settings.apiEnabled && isConnected && (
                    <div className="alert-box success">
                      <CheckCircle2 size={14} />
                      <span>
                        API active at <code style={{ fontFamily: 'var(--font-mono)', fontSize: '11px', background: 'rgba(255,255,255,0.08)', padding: '0 4px', borderRadius: '3px' }}>
                          http://127.0.0.1:{settings.apiPort}
                        </code>
                        {' — open in '}
                        <a href={`http://yacd.haishan.me/?hostname=127.0.0.1&port=${settings.apiPort}&secret=${settings.apiSecret}`} target="_blank" rel="noreferrer" style={{ color: 'var(--accent-primary)', textDecoration: 'underline' }}>
                          Yacd UI <ExternalLink size={10} style={{ display: 'inline' }} />
                        </a>
                      </span>
                    </div>
                  )}
                </div>
              </div>
              <div className="info-box">
                <Info size={13} />
                <span>The Clash-compatible API lets Yacd or MetaCubeX dashboard control routing and monitor connections in real-time.</span>
              </div>
            </div>
          )}
        </div>
      </div>
    </ViewShell>
  );
}
