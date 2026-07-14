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
import { Card, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { FormInput, FormSelect, FormSwitchRow, SegmentedControl } from '@/components/form';

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
      <div className="grid grid-cols-1 md:grid-cols-[160px_1fr] gap-6 overflow-hidden flex-1 min-h-0">
        {/* Secondary nav */}
        <div className="flex flex-row md:flex-col gap-1 overflow-x-auto md:overflow-x-visible shrink-0 pb-1 md:pb-0">
          {sideNavItems.map(([id, Icon, label]) => (
            <Button
              key={id}
              variant="ghost"
              className={`h-8 justify-start gap-2 px-3 text-xs font-bold rounded ${
                settingsTab === id
                  ? 'bg-muted text-foreground'
                  : 'text-muted-foreground hover:text-foreground hover:bg-muted/40'
              }`}
              onClick={() => setSettingsTab(id)}
            >
              <Icon className="size-3.5 shrink-0" />
              <span>{label}</span>
            </Button>
          ))}
        </div>

        {/* Content */}
        <div className="overflow-y-auto pr-1">

          {/* ── GENERAL ── */}
          {settingsTab === 'general' && (
            <div className="flex flex-col gap-4">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div className="flex flex-col gap-4">
                  <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-3">
                    <CardHeader className="p-0 pb-1 shrink-0">
                      <CardTitle className="text-xs font-bold text-foreground">Proxy Mode</CardTitle>
                    </CardHeader>
                    <FormSwitchRow title="Proxy Mode: TUN vs System Proxy" desc="Toggle between native routing (TUN) and OS settings (System Proxy)."
                      checked={settings.proxyMode === 'tun'} onChange={(v) => updateSettings({ proxyMode: v ? 'tun' : 'system' })} />
                  </Card>

                  <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-3">
                    <CardHeader className="p-0 pb-1 shrink-0">
                      <CardTitle className="text-xs font-bold text-foreground">App Preferences</CardTitle>
                    </CardHeader>
                    <FormSwitchRow title="Close to Tray" desc="Minimize to system tray on close" checked={settings.closeToTray} onChange={(v) => updateSettings({ closeToTray: v })} />
                    <FormSwitchRow title="Launch on System Login" desc="Launch minimized on user login" checked={settings.autostart} onChange={(v) => updateSettings({ autostart: v })} />
                    <FormSwitchRow title="LAN Hotspot Sharing" desc="Bind to 0.0.0.0 for LAN device sharing" checked={settings.wifiSharing} onChange={(v) => updateSettings({ wifiSharing: v })} />
                  </Card>

                  <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-3">
                    <CardHeader className="p-0 pb-1 shrink-0">
                      <CardTitle className="text-xs font-bold text-foreground">Connection Optimization</CardTitle>
                    </CardHeader>
                    <FormSwitchRow title="Enable Multiplexing (Mux)" desc="Multiplexes multiple streams over a single connection" checked={settings.multiplex} onChange={(v) => updateSettings({ multiplex: v })} />
                  </Card>

                  <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-3">
                    <CardHeader className="p-0 pb-1 shrink-0">
                      <CardTitle className="text-xs font-bold text-foreground">Appearance</CardTitle>
                    </CardHeader>
                    <SegmentedControl
                      value={theme}
                      onChange={(mode) => setTheme(mode as ThemeMode)}
                      options={[
                        { value: 'light', label: 'Light', icon: Sun },
                        { value: 'dark', label: 'Dark', icon: Moon },
                        { value: 'system', label: 'System', icon: Monitor },
                      ]}
                      className="w-full justify-stretch"
                    />
                  </Card>
                </div>

                <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-3 h-fit">
                  <CardHeader className="p-0 pb-1 shrink-0">
                    <CardTitle className="text-xs font-bold text-foreground">Inbound Ports</CardTitle>
                  </CardHeader>
                  <FormSwitchRow title="Use Separate HTTP & SOCKS5 Ports" desc="Bind separate HTTP and SOCKS5 protocol listeners"
                    checked={settings.useSeparatePorts} onChange={(v) => updateSettings({ useSeparatePorts: v })} />
                  <div style={{ opacity: settings.useSeparatePorts ? 1 : 0.4, pointerEvents: settings.useSeparatePorts ? 'auto' : 'none' }}>
                    <div className="grid grid-cols-2 gap-2.5">
                      {[
                        { label: 'HTTP Proxy Port', key: 'httpPort' as keyof Settings, val: httpPort, set: (v: number) => setPorts({ httpPort: v }) },
                        { label: 'SOCKS5 Port', key: 'socksPort' as keyof Settings, val: socksPort, set: (v: number) => setPorts({ socksPort: v }) },
                      ].map(({ label, key, val, set }) => (
                        <div key={key} className="flex flex-col gap-1">
                          <div className="flex justify-between items-center px-0.5">
                            <label className="text-[9.5px] font-bold text-muted-foreground uppercase">{label}</label>
                            <span className={`text-[9px] font-bold ${conflictingPorts.includes(val) ? 'text-destructive' : 'text-foreground/60'}`}>
                              {conflictingPorts.includes(val) ? '● Conflict' : '● OK'}
                            </span>
                          </div>
                          <FormInput
                            type="number"
                            mono
                            className="h-8"
                            style={{ borderColor: conflictingPorts.includes(val) ? 'var(--destructive)' : undefined }}
                            value={val}
                            onChange={(v) => { const numVal = parseInt(v) || 0; set(numVal); updateSettings({ [key]: numVal } as Partial<Settings>); }}
                          />
                        </div>
                      ))}
                    </div>
                  </div>
                  <div style={{ opacity: !settings.useSeparatePorts ? 1 : 0.4, pointerEvents: !settings.useSeparatePorts ? 'auto' : 'none' }} className="mt-1">
                    <div className="flex flex-col gap-1">
                      <div className="flex justify-between items-center px-0.5">
                        <label className="text-[9.5px] font-bold text-muted-foreground uppercase">Mixed Inbound Port (Recommended)</label>
                        <span className={`text-[9px] font-bold ${conflictingPorts.includes(mixedPort) ? 'text-destructive' : 'text-foreground/60'}`}>
                          {conflictingPorts.includes(mixedPort) ? '● Conflict' : '● OK'}
                        </span>
                      </div>
                      <FormInput
                        type="number"
                        mono
                        className="h-8"
                        style={{ borderColor: conflictingPorts.includes(mixedPort) ? 'var(--destructive)' : undefined }}
                        value={mixedPort}
                        onChange={(v) => { const numVal = parseInt(v) || 0; setPorts({ mixedPort: numVal }); updateSettings({ mixedPort: numVal }); }}
                      />
                    </div>
                  </div>
                  <p className="text-[10px] text-muted-foreground/80 mt-1">Port changes take effect on next connection restart.</p>
                </Card>
              </div>

              <Card className="flex items-center gap-3.5 p-3.5 px-4 bg-card border-border shadow-sm shrink-0">
                <img src={xLinkLogo} alt="X-Link" className="size-10 rounded-lg shrink-0" />
                <div className="flex flex-col">
                  <h4 className="text-xs font-bold text-foreground">X-Link</h4>
                  <p className="text-[10px] text-muted-foreground mt-0.5">
                    Empowered by sing-box
                  </p>
                </div>
                <div className="ml-auto">
                  <Badge variant="outline" className="h-5 px-2.5 text-[10px] font-semibold border-border bg-muted/30">v{appVersion}</Badge>
                </div>
              </Card>
            </div>
          )}

          {/* ── TUN MODE ── */}
          {settingsTab === 'tun' && (
            <div className="flex flex-col gap-4">
              {!isElevated && (
                <div className="bg-destructive/10 text-destructive text-[11px] px-3.5 py-2.5 rounded border border-destructive/20 flex items-center gap-2">
                  <AlertTriangle className="size-3.5 shrink-0" />
                  <span>TUN Mode requires Administrator elevation. Click "Run as Admin" on the Dashboard.</span>
                </div>
              )}
              <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-4">
                <CardHeader className="p-0 pb-1 shrink-0">
                  <CardTitle className="text-xs font-bold text-foreground">TUN Interface Configuration</CardTitle>
                </CardHeader>
                <div className="grid grid-cols-2 gap-3.5">
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider">TUN Stack Implementation</label>
                    <FormSelect value={settings.tunStack} onChange={(v) => updateSettings({ tunStack: v as Settings['tunStack'] })} options={[
                      { value: 'mixed', label: 'Mixed (gVisor TCP + system UDP)' },
                      { value: 'gvisor', label: 'gVisor (Full userspace)' },
                      { value: 'system', label: 'System (Kernel-level)' },
                    ]} />
                    <span className="text-[10px] text-muted-foreground/80">Mixed: best compatibility. gVisor: best isolation.</span>
                  </div>
                  <FormInput
                    label="TUN MTU"
                    type="number"
                    mono
                    className="h-8"
                    value={settings.tunMtu}
                    onChange={(v) => updateSettings({ tunMtu: parseInt(v) || 1400 })}
                  />
                  <span className="text-[10px] text-muted-foreground/80">1400 recommended. Use 1500 for standard ethernet.</span>
                </div>
                {([
                  { key: 'tunAutoRoute', title: 'Auto Route', desc: 'Add routes to direct traffic into TUN interface' },
                  { key: 'tunAutoRedirect', title: 'Auto Redirect (nftables)', desc: 'Use nftables-based redirect on Linux' },
                  { key: 'tunStrictRoute', title: 'Strict Route', desc: 'Block unmatched traffic to prevent leaks' },
                  { key: 'tunEndpointIndependentNat', title: 'Endpoint Independent NAT', desc: 'Improves UDP NAT traversal (P2P, gaming)' },
                ] as const).map(({ key, title, desc }) => (
                  <FormSwitchRow key={key} title={title} desc={desc} checked={settings[key]} onChange={(v) => updateSettings({ [key]: v } as Partial<Settings>)} disabled={!isElevated} />
                ))}
              </Card>
              <div className="info-box bg-muted/40 border border-border/40 p-2.5 px-3 rounded-lg flex items-start gap-2 text-[10.5px] text-muted-foreground leading-relaxed shrink-0">
                <Info className="size-3.5 mt-0.5 text-foreground shrink-0" />
                <span>TUN mode captures ALL system traffic. Combined with Auto Route + Strict Route, it prevents DNS leaks.</span>
              </div>
            </div>
          )}

          {/* ── SNIFFING ── */}
          {settingsTab === 'sniff' && (
            <div className="flex flex-col gap-4">
              <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-4">
                <CardHeader className="p-0 pb-1 shrink-0">
                  <CardTitle className="text-xs font-bold text-foreground">Traffic Sniffing & Protocol Detection</CardTitle>
                </CardHeader>
                <FormSwitchRow title="Enable Traffic Sniffing" desc="Inspect connection metadata to extract domain names for routing"
                  checked={settings.sniffEnabled} onChange={(v) => updateSettings({ sniffEnabled: v })} />
                <div style={{ opacity: settings.sniffEnabled ? 1 : 0.4, pointerEvents: settings.sniffEnabled ? 'auto' : 'none' }}>
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                    {([
                      { key: 'sniffHttp', title: 'HTTP Sniffing', desc: 'Extract hostname from HTTP Host header' },
                      { key: 'sniffTls', title: 'TLS/HTTPS Sniffing', desc: 'Extract SNI from TLS ClientHello' },
                      { key: 'sniffQuic', title: 'QUIC/HTTP3 Sniffing', desc: 'Extract SNI from QUIC Initial packets' },
                      { key: 'sniffOverrideDestination', title: 'Override Destination', desc: 'Use sniffed domain to override target IP' },
                    ] as const).map(({ key, title, desc }) => (
                      <FormSwitchRow key={key} title={title} desc={desc} checked={settings[key]} onChange={(v) => updateSettings({ [key]: v } as Partial<Settings>)} />
                    ))}
                  </div>
                </div>
              </Card>
            </div>
          )}

          {/* ── DNS ── */}
          {settingsTab === 'dns' && (
            <div className="flex flex-col gap-4">
              <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-4">
                <CardHeader className="p-0 pb-1 shrink-0">
                  <CardTitle className="text-xs font-bold text-foreground">DNS Servers</CardTitle>
                </CardHeader>
                <div className="grid grid-cols-1 sm:grid-cols-3 gap-3.5">
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider">Primary DNS (via proxy)</label>
                    <FormInput value={settings.primaryDns} onChange={(v) => updateSettings({ primaryDns: v })} placeholder="https://1.1.1.1/dns-query" mono />
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider">Fallback DNS (via proxy)</label>
                    <FormInput value={settings.fallbackDns} onChange={(v) => updateSettings({ fallbackDns: v })} placeholder="https://8.8.8.8/dns-query" mono />
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider">Direct DNS (Local/Bypass)</label>
                    <FormInput value={settings.directDns} onChange={(v) => updateSettings({ directDns: v })} placeholder="Auto (System DNS)" mono />
                  </div>
                </div>
                <p className="text-[10px] text-muted-foreground/80">
                  Used for resolving domains. The direct DNS server is used to resolve direct/bypassed connections.
                </p>
              </Card>

              <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-4">
                <CardHeader className="p-0 pb-1 shrink-0">
                  <CardTitle className="text-xs font-bold text-foreground">DNS Options & Strategy</CardTitle>
                </CardHeader>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider">DNS Query Strategy</label>
                    <FormSelect value={settings.dnsStrategy} onChange={(v) => updateSettings({ dnsStrategy: v as Settings['dnsStrategy'] })} options={[
                      { value: 'prefer_ipv4', label: 'Prefer IPv4 (Default)' },
                      { value: 'ipv4_only', label: 'IPv4 Only' },
                      { value: 'prefer_ipv6', label: 'Prefer IPv6' },
                      { value: 'ipv6_only', label: 'IPv6 Only' },
                    ]} />
                  </div>
                  <div className="flex flex-col gap-3">
                    <FormSwitchRow title="Enable DNS Caching" desc="Store resolved domain IPs locally to speed up browsing"
                      checked={settings.dnsCaching} onChange={(v) => updateSettings({ dnsCaching: v })} />
                    <FormSwitchRow title="DNS Leak Protection" desc="Block DNS requests outside the secure tunnel"
                      checked={settings.dnsLeakProtection} onChange={(v) => updateSettings({ dnsLeakProtection: v })} />
                  </div>
                </div>
              </Card>

              <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-4">
                <CardHeader className="p-0 pb-1 shrink-0">
                  <CardTitle className="text-xs font-bold text-foreground">Resolution Mode</CardTitle>
                </CardHeader>
                <div className="flex flex-col gap-1.5">
                  <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider">DNS Mode</label>
                  <FormSelect value={settings.dnsMode} onChange={(v) => updateSettings({ dnsMode: v as DnsMode })} options={[
                    { value: 'fakeip', label: 'FakeIP (recommended for TUN)' },
                    { value: 'normal', label: 'Normal (real IP resolution)' },
                  ]} />
                  <span className="text-[10px] text-muted-foreground/80">
                    FakeIP avoids leaking real destination IPs to the local resolver and speeds up routing decisions.
                  </span>
                </div>
                <div style={{ opacity: settings.dnsMode === 'fakeip' ? 1 : 0.4, pointerEvents: settings.dnsMode === 'fakeip' ? 'auto' : 'none' }}>
                  <div className="grid grid-cols-2 gap-3.5">
                    <div className="flex flex-col gap-1.5">
                      <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider">FakeIP Range</label>
                      <FormInput value={settings.fakeipRange} onChange={(v) => updateSettings({ fakeipRange: v })} placeholder="198.18.0.0/15" mono />
                    </div>
                    <div className="flex flex-col gap-1.5">
                      <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider">FakeIP Exclude Filter</label>
                      <FormInput value={settings.fakeipFilter} onChange={(v) => updateSettings({ fakeipFilter: v })} placeholder="geosite:private" mono />
                    </div>
                  </div>
                </div>
              </Card>

              <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-3">
                <CardHeader className="p-0 pb-1 shrink-0">
                  <CardTitle className="text-xs font-bold text-foreground">Routing</CardTitle>
                </CardHeader>
                <FormSwitchRow title="Bypass LAN / Private IPs" desc="Route RFC-1918 addresses directly instead of through the proxy"
                  checked={settings.bypassLan} onChange={(v) => updateSettings({ bypassLan: v })} />
              </Card>

              <div className="info-box bg-muted/40 border border-border/40 p-2.5 px-3 rounded-lg flex items-start gap-2 text-[10.5px] text-muted-foreground leading-relaxed shrink-0">
                <Info className="size-3.5 mt-0.5 text-foreground shrink-0" />
                <span>DNS settings only take effect in TUN mode and apply on the next connection.</span>
              </div>
            </div>
          )}

          {/* ── API ── */}
          {settingsTab === 'api' && (
            <div className="flex flex-col gap-4">
              <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-4">
                <CardHeader className="p-0 pb-1 shrink-0">
                  <CardTitle className="text-xs font-bold text-foreground">Clash-Compatible REST API</CardTitle>
                </CardHeader>
                <FormSwitchRow title="Enable External API" desc="Expose HTTP API for external controllers (Yacd, MetaCubeX)"
                  checked={settings.apiEnabled} onChange={(v) => updateSettings({ apiEnabled: v })} />
                <div style={{ opacity: settings.apiEnabled ? 1 : 0.4, pointerEvents: settings.apiEnabled ? 'auto' : 'none' }} className="flex flex-col gap-4">
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-3.5">
                    <FormInput
                      label="API Port"
                      type="number"
                      mono
                      className="h-8"
                      value={settings.apiPort}
                      onChange={(v) => updateSettings({ apiPort: parseInt(v) || 9090 })}
                    />
                    <div className="flex flex-col gap-1.5">
                      <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider">API Secret Token</label>
                      <div className="flex gap-2">
                        <div className="flex-1">
                          <FormInput value={settings.apiSecret} onChange={(v) => updateSettings({ apiSecret: v })} placeholder="Leave empty for no auth" mono className="h-8" />
                        </div>
                        <Button
                          type="button"
                          variant="outline"
                          size="icon"
                          className="size-8 shrink-0"
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
                          <RefreshCw className="size-3.5" />
                        </Button>
                      </div>
                    </div>
                  </div>
                  <FormSwitchRow title="Allow CORS (Cross-Origin)" desc="Allow web-based controllers (Yacd) to connect from browser"
                    checked={settings.apiCors} onChange={(v) => updateSettings({ apiCors: v })} />
                  {settings.apiEnabled && isConnected && (
                    <div className="bg-muted/80 text-foreground text-[11px] px-3.5 py-2.5 rounded border border-border/50 flex items-center gap-2">
                      <CheckCircle2 className="size-3.5 text-muted-foreground shrink-0" />
                      <span className="leading-normal">
                        API active at <code className="font-mono text-[10.5px] bg-background border border-border px-1 py-0.5 rounded text-foreground">
                          http://127.0.0.1:{settings.apiPort}
                        </code>
                        {' — open in '}
                        <a href={`http://yacd.haishan.me/?hostname=127.0.0.1&port=${settings.apiPort}&secret=${settings.apiSecret}`} target="_blank" rel="noreferrer" className="text-foreground font-semibold underline underline-offset-2">
                          Yacd UI <ExternalLink className="size-3 inline-block ml-0.5" />
                        </a>
                      </span>
                    </div>
                  )}
                </div>
              </Card>
              <div className="info-box bg-muted/40 border border-border/40 p-2.5 px-3 rounded-lg flex items-start gap-2 text-[10.5px] text-muted-foreground leading-relaxed shrink-0">
                <Info className="size-3.5 mt-0.5 text-foreground shrink-0" />
                <span>The Clash-compatible API lets Yacd or MetaCubeX dashboard control routing and monitor connections in real-time.</span>
              </div>
            </div>
          )}
        </div>
      </div>
    </ViewShell>
  );
}
