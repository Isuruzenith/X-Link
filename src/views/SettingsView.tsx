import React from 'react';
import {
  Settings as SettingsIcon, Network, Eye, Dna, Code2,
  Info, AlertTriangle, CheckCircle2, ExternalLink, Sun, Moon, Monitor,
  RefreshCw
} from 'lucide-react';
import { ViewShell } from '../components/ViewShell';
import { useSettingsStore, type ThemeMode } from '../stores/settingsStore';
import { useConnectionStore } from '../stores/connectionStore';
import type { Settings, DnsMode } from '../utils/store';
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

type SettingsSection = 'general' | 'tun' | 'sniff' | 'dns' | 'api';

export function SettingsView() {
  const {
    settings,
    conflictingPorts,
    isElevated,
    appVersion,
    theme,
    updateSettings,
    setTheme,
  } = useSettingsStore();

  const {
    isConnected,
    httpPort,
    socksPort,
    mixedPort,
    setPorts,
  } = useConnectionStore();

  const [settingsTab, setSettingsTab] = React.useState<SettingsSection>('general');

  const sideNavItems: [SettingsSection, React.ComponentType<{ className?: string; size?: number }>, string][] = [
    ['general', SettingsIcon, 'General'],
    ['tun', Network, 'TUN Mode'],
    ['sniff', Eye, 'Sniffing'],
    ['dns', Dna, 'DNS'],
    ['api', Code2, 'API'],
  ];

  return (
    <ViewShell title="Settings" subtitle="Proxy mode, TUN, sniffing, DNS and API options">
      <div className="settings-layout" style={{ height: 'calc(100vh - 100px)' }}>
        {/* Secondary nav */}
        <div className="settings-sidenav">
          {sideNavItems.map(([id, Icon, label]) => (
            <div key={id} className={`settings-nav-item ${settingsTab === id ? 'active' : ''}`} onClick={() => setSettingsTab(id)}>
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
                      checked={settings.proxyMode === 'tun'} onChange={(v) => updateSettings({ proxyMode: v ? 'tun' : 'system' })} />
                  </div>

                  <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                    <div className="settings-section-heading">App Preferences</div>
                    <SwitchRow title="Close to Tray" desc="Minimize to system tray on close" checked={settings.closeToTray} onChange={(v) => updateSettings({ closeToTray: v })} />
                    <SwitchRow title="Launch on System Login" desc="Launch minimized on user login" checked={settings.autostart} onChange={(v) => updateSettings({ autostart: v })} />
                    <SwitchRow title="LAN Hotspot Sharing" desc="Bind to 0.0.0.0 for LAN device sharing" checked={settings.wifiSharing} onChange={(v) => updateSettings({ wifiSharing: v })} />
                  </div>

                  <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                    <div className="settings-section-heading">Connection Optimization</div>
                    <SwitchRow title="Enable Multiplexing (Mux)" desc="Multiplexes multiple streams over a single connection to eliminate handshake latency." checked={settings.multiplex} onChange={(v) => updateSettings({ multiplex: v })} />
                  </div>

                  <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                    <div className="settings-section-heading">Appearance</div>
                    <div style={{ display: 'flex', gap: '6px' }}>
                      {([['dark', Moon, 'Dark'], ['light', Sun, 'Light'], ['system', Monitor, 'System']] as const).map(([mode, Icon, label]) => (
                        <button
                          key={mode}
                          onClick={() => setTheme(mode as ThemeMode)}
                          style={{
                            flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '6px',
                            padding: '8px 12px', borderRadius: 'var(--r-md)', border: 'none', cursor: 'pointer',
                            fontSize: '12px', fontWeight: 500, transition: 'all 0.15s',
                            background: theme === mode ? 'var(--accent-primary)' : 'var(--surface-sunken)',
                            color: theme === mode ? 'var(--text-on-accent)' : 'var(--text-low)',
                          }}
                        >
                          <Icon size={14} />{label}
                        </button>
                      ))}
                    </div>
                  </div>
                </div>

                <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                  <div className="settings-section-heading">Inbound Ports</div>
                  <SwitchRow title="Use Separate HTTP & SOCKS5 Ports" desc="Bind separate HTTP and SOCKS5 protocol listeners instead of a single mixed port"
                    checked={settings.useSeparatePorts} onChange={(v) => updateSettings({ useSeparatePorts: v })} />
                  <div style={{ opacity: settings.useSeparatePorts ? 1 : 0.4, pointerEvents: settings.useSeparatePorts ? 'auto' : 'none' }}>
                    <div className="grid-2">
                      {[
                        { label: 'HTTP Proxy Port', key: 'httpPort' as keyof Settings, val: httpPort, set: (v: number) => setPorts({ httpPort: v }) },
                        { label: 'SOCKS5 Port', key: 'socksPort' as keyof Settings, val: socksPort, set: (v: number) => setPorts({ socksPort: v }) },
                      ].map(({ label, key, val, set }) => (
                        <div key={key} className="form-group">
                          <div className="flex-row-between">
                            <label className="form-label">{label}</label>
                            <span style={{ fontSize: '10px', fontWeight: 600, color: conflictingPorts.includes(val) ? 'var(--status-err)' : 'var(--status-ok)' }}>
                              {conflictingPorts.includes(val) ? '● Conflict' : '● OK'}
                            </span>
                          </div>
                          <input type="number" className="text-input" style={{ borderColor: conflictingPorts.includes(val) ? 'var(--status-err)' : undefined }}
                            value={val} onChange={(e) => { const v = parseInt(e.target.value) || 0; set(v); updateSettings({ [key]: v } as Partial<Settings>); }} />
                        </div>
                      ))}
                    </div>
                  </div>
                  <div style={{ opacity: !settings.useSeparatePorts ? 1 : 0.4, pointerEvents: !settings.useSeparatePorts ? 'auto' : 'none', marginTop: '4px' }}>
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <div className="flex-row-between">
                        <label className="form-label">Mixed Inbound Port (Recommended)</label>
                        <span style={{ fontSize: '10px', fontWeight: 600, color: conflictingPorts.includes(mixedPort) ? 'var(--status-err)' : 'var(--status-ok)' }}>
                          {conflictingPorts.includes(mixedPort) ? '● Conflict' : '● OK'}
                        </span>
                      </div>
                      <input type="number" className="text-input" style={{ borderColor: conflictingPorts.includes(mixedPort) ? 'var(--status-err)' : undefined }}
                        value={mixedPort} onChange={(e) => { const v = parseInt(e.target.value) || 0; setPorts({ mixedPort: v }); updateSettings({ mixedPort: v }); }} />
                    </div>
                  </div>
                  <p style={{ fontSize: '11px', color: 'var(--text-low)', marginTop: '8px' }}>Port changes take effect on next connection restart.</p>
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
                    <Sel value={settings.tunStack} onChange={(v) => updateSettings({ tunStack: v as Settings['tunStack'] })} options={[
                      { value: 'mixed', label: 'Mixed (gVisor TCP + system UDP)' },
                      { value: 'gvisor', label: 'gVisor (Full userspace)' },
                      { value: 'system', label: 'System (Kernel-level)' },
                    ]} />
                    <span style={{ fontSize: '11px', color: 'var(--text-low)', marginTop: '4px', display: 'block' }}>Mixed: best compatibility. gVisor: best isolation.</span>
                  </div>
                  <div className="form-group" style={{ marginBottom: 0 }}>
                    <label className="form-label">TUN MTU</label>
                    <input type="number" className="text-input" value={settings.tunMtu} onChange={(e) => updateSettings({ tunMtu: parseInt(e.target.value) || 1400 })} />
                    <span style={{ fontSize: '11px', color: 'var(--text-low)', marginTop: '4px', display: 'block' }}>1400 recommended to prevent packet fragmentation. Use 1500 for standard ethernet.</span>
                  </div>
                </div>
                {([
                  { key: 'tunAutoRoute', title: 'Auto Route', desc: 'Add routes to direct traffic into TUN interface' },
                  { key: 'tunAutoRedirect', title: 'Auto Redirect (nftables)', desc: 'Use nftables-based redirect on Linux' },
                  { key: 'tunStrictRoute', title: 'Strict Route', desc: 'Block unmatched traffic to prevent leaks' },
                  { key: 'tunEndpointIndependentNat', title: 'Endpoint Independent NAT', desc: 'Improves UDP NAT traversal (P2P, gaming)' },
                ] as const).map(({ key, title, desc }) => (
                  <SwitchRow key={key} title={title} desc={desc} checked={settings[key]} onChange={(v) => updateSettings({ [key]: v } as Partial<Settings>)} disabled={!isElevated} />
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
                  checked={settings.sniffEnabled} onChange={(v) => updateSettings({ sniffEnabled: v })} />
                <div style={{ opacity: settings.sniffEnabled ? 1 : 0.4, pointerEvents: settings.sniffEnabled ? 'auto' : 'none' }}>
                  <div className="grid-2">
                    {([
                      { key: 'sniffHttp', title: 'HTTP Sniffing', desc: 'Extract hostname from HTTP Host header' },
                      { key: 'sniffTls', title: 'TLS/HTTPS Sniffing', desc: 'Extract SNI from TLS ClientHello' },
                      { key: 'sniffQuic', title: 'QUIC/HTTP3 Sniffing', desc: 'Extract SNI from QUIC Initial packets' },
                      { key: 'sniffOverrideDestination', title: 'Override Destination', desc: 'Use sniffed domain to override target IP' },
                    ] as const).map(({ key, title, desc }) => (
                      <SwitchRow key={key} title={title} desc={desc} checked={settings[key]} onChange={(v) => updateSettings({ [key]: v } as Partial<Settings>)} />
                    ))}
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* ── DNS ── */}
          {settingsTab === 'dns' && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
              <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                <div className="settings-section-heading">DNS Servers</div>
                <div className="grid-2" style={{ gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))' }}>
                  <div className="form-group" style={{ marginBottom: 0 }}>
                    <label className="form-label">Primary DNS (via proxy)</label>
                    <Inp value={settings.primaryDns} onChange={(v) => updateSettings({ primaryDns: v })} placeholder="https://1.1.1.1/dns-query" mono />
                  </div>
                  <div className="form-group" style={{ marginBottom: 0 }}>
                    <label className="form-label">Fallback DNS (via proxy)</label>
                    <Inp value={settings.fallbackDns} onChange={(v) => updateSettings({ fallbackDns: v })} placeholder="https://8.8.8.8/dns-query" mono />
                  </div>
                  <div className="form-group" style={{ marginBottom: 0 }}>
                    <label className="form-label">Direct DNS (Local/Bypass)</label>
                    <Inp value={settings.directDns} onChange={(v) => updateSettings({ directDns: v })} placeholder="223.5.5.5" mono />
                  </div>
                </div>
                <p style={{ fontSize: '11px', color: 'var(--text-low)' }}>
                  Used for resolving domains. The direct DNS server is used to resolve direct/bypassed connections.
                </p>
              </div>

              <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                <div className="settings-section-heading">DNS Options & Strategy</div>
                <div className="grid-2">
                  <div className="form-group" style={{ marginBottom: 0 }}>
                    <label className="form-label">DNS Query Strategy</label>
                    <Sel value={settings.dnsStrategy} onChange={(v) => updateSettings({ dnsStrategy: v as Settings['dnsStrategy'] })} options={[
                      { value: 'ipv4_only', label: 'IPv4 Only (Default)' },
                      { value: 'prefer_ipv4', label: 'Prefer IPv4' },
                      { value: 'prefer_ipv6', label: 'Prefer IPv6' },
                      { value: 'ipv6_only', label: 'IPv6 Only' },
                    ]} />
                  </div>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                    <SwitchRow title="Enable DNS Caching" desc="Store resolved domain IPs locally to speed up browsing"
                      checked={settings.dnsCaching} onChange={(v) => updateSettings({ dnsCaching: v })} />
                    <SwitchRow title="DNS Leak Protection" desc="Block DNS requests outside the secure tunnel"
                      checked={settings.dnsLeakProtection} onChange={(v) => updateSettings({ dnsLeakProtection: v })} />
                  </div>
                </div>
              </div>

              <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                <div className="settings-section-heading">Resolution Mode</div>
                <div className="form-group" style={{ marginBottom: 0 }}>
                  <label className="form-label">DNS Mode</label>
                  <Sel value={settings.dnsMode} onChange={(v) => updateSettings({ dnsMode: v as DnsMode })} options={[
                    { value: 'fakeip', label: 'FakeIP (recommended for TUN)' },
                    { value: 'normal', label: 'Normal (real IP resolution)' },
                  ]} />
                  <span style={{ fontSize: '11px', color: 'var(--text-low)', marginTop: '4px', display: 'block' }}>
                    FakeIP avoids leaking real destination IPs to the local resolver and speeds up routing decisions.
                  </span>
                </div>
                <div style={{ opacity: settings.dnsMode === 'fakeip' ? 1 : 0.4, pointerEvents: settings.dnsMode === 'fakeip' ? 'auto' : 'none' }}>
                  <div className="grid-2">
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">FakeIP Range</label>
                      <Inp value={settings.fakeipRange} onChange={(v) => updateSettings({ fakeipRange: v })} placeholder="198.18.0.0/15" mono />
                    </div>
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">FakeIP Exclude Filter</label>
                      <Inp value={settings.fakeipFilter} onChange={(v) => updateSettings({ fakeipFilter: v })} placeholder="geosite:private" mono />
                    </div>
                  </div>
                </div>
              </div>

              <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                <div className="settings-section-heading">Routing</div>
                <SwitchRow title="Bypass LAN / Private IPs" desc="Route RFC-1918 addresses directly instead of through the proxy"
                  checked={settings.bypassLan} onChange={(v) => updateSettings({ bypassLan: v })} />
              </div>

              <div className="info-box">
                <Info size={13} />
                <span>DNS settings only take effect in TUN mode and apply on the next connection.</span>
              </div>
            </div>
          )}

          {/* ── API ── */}
          {settingsTab === 'api' && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
              <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                <div className="settings-section-heading">Clash-Compatible REST API</div>
                <SwitchRow title="Enable External API" desc="Expose HTTP API for external controllers (Yacd, MetaCubeX)"
                  checked={settings.apiEnabled} onChange={(v) => updateSettings({ apiEnabled: v })} />
                <div style={{ opacity: settings.apiEnabled ? 1 : 0.4, pointerEvents: settings.apiEnabled ? 'auto' : 'none', display: 'flex', flexDirection: 'column', gap: '16px' }}>
                  <div className="grid-2">
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">API Port</label>
                      <input type="number" className="text-input" value={settings.apiPort} onChange={(e) => updateSettings({ apiPort: parseInt(e.target.value) || 9090 })} />
                    </div>
                    <div className="form-group" style={{ marginBottom: 0 }}>
                      <label className="form-label">API Secret Token</label>
                      <div style={{ display: 'flex', gap: '8px' }}>
                        <div style={{ flex: 1 }}>
                          <Inp value={settings.apiSecret} onChange={(v) => updateSettings({ apiSecret: v })} placeholder="Leave empty for no auth" mono />
                        </div>
                        <button
                          type="button"
                          className="btn secondary"
                          style={{ padding: '0 10px', height: '32px', display: 'flex', alignItems: 'center', justifyContent: 'center' }}
                          title="Generate new random API Secret Token"
                          onClick={() => {
                            const chars = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
                            let secret = '';
                            for (let i = 0; i < 16; i++) {
                              secret += chars.charAt(Math.floor(Math.random() * chars.length));
                            }
                            updateSettings({ apiSecret: secret });
                          }}
                        >
                          <RefreshCw size={14} />
                        </button>
                      </div>
                    </div>
                  </div>
                  <SwitchRow title="Allow CORS (Cross-Origin)" desc="Allow web-based controllers (Yacd) to connect from browser"
                    checked={settings.apiCors} onChange={(v) => updateSettings({ apiCors: v })} />
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
