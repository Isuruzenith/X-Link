import React from 'react';
import { Icon } from '@/components/ui/icon';
import { ViewShell } from '../components/ViewShell';
import { useSettingsStore, type ThemeMode } from '../stores/settingsStore';
import { useConnectionStore } from '../stores/connectionStore';
import { useToastStore } from '../stores/toastStore';
import { useProfileStore } from '../stores/profileStore';
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
    portStatuses,
    isElevated,
    appVersion,
    theme,
    updateSettings,
    setTheme,
  } = useSettingsStore();
  const clearNodeGeoCache = useProfileStore((state) => state.clearNodeGeoCache);
  const {
    connectionStatus,
    isConnected,
    httpPort,
    socksPort,
    mixedPort,
    setPorts,
  } = useConnectionStore();
  const connectedConfigChange = connectionStatus === 'connected';

  const [settingsTab, setSettingsTab] = React.useState<SettingsSection>('general');

  const sideNavItems: [SettingsSection, string, string][] = [
    ['general', 'tune', 'General'],
    ['tun', 'router', 'TUN Mode'],
    ['sniff', 'visibility', 'Sniffing'],
    ['dns', 'dns', 'DNS'],
    ['api', 'code', 'API'],
  ];

  return (
    <ViewShell title="Settings" subtitle="Proxy mode, TUN, sniffing, DNS and API options">
      <div className="settings-layout">
        {/* Secondary nav */}
        <nav className="settings-navigation" aria-label="Settings sections">
          {sideNavItems.map(([id, iconName, label]) => (
            <Button
              key={id}
              variant="ghost"
              className={`settings-nav-item ${settingsTab === id ? 'is-active' : ''}`}
              aria-current={settingsTab === id ? 'page' : undefined}
              onClick={() => setSettingsTab(id)}
            >
              <Icon name={iconName} size={15} className="shrink-0" />
              <span>{label}</span>
            </Button>
          ))}
        </nav>

        {/* Content */}
        <div className="settings-content" id={`settings-${settingsTab}`} role="region" aria-label={`${sideNavItems.find(([id]) => id === settingsTab)?.[2]} settings`}>

          {/* ── GENERAL ── */}
          {settingsTab === 'general' && (
            <div className="flex flex-col gap-4">
              <div className="grid grid-cols-1 xl:grid-cols-2 gap-4">
                <div className="flex flex-col gap-4">
                  <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-3">
                    <CardHeader className="p-0 pb-1 shrink-0 flex flex-row items-center justify-between space-y-0">
                      <CardTitle className="text-xs font-bold text-foreground">Proxy Mode</CardTitle>
                      <Badge variant="outline" className="h-4.5 px-1.5 text-[8.5px] font-bold text-orange-500 border-orange-500/20 bg-orange-500/5 select-none">Next connect</Badge>
                    </CardHeader>
                    <FormSwitchRow title="Proxy Mode: TUN vs System Proxy" desc="Toggle between native routing (TUN) and OS settings (System Proxy)."
                      checked={settings.proxyMode === 'tun'}
                      onChange={(v) => {
                        if (v) {
                          useToastStore.getState().addToast(
                            'warning',
                            'TUN Mode requires Administrator privileges and the Wintun driver to be installed/loaded.',
                            'TUN Elevation Alert'
                          );
                        }
                        updateSettings({ proxyMode: v ? 'tun' : 'system' });
                      }}
                    />
                    {connectedConfigChange && <p className="text-[10px] text-amber-500">Proxy mode is a saved preference; the running session keeps its current mode until you reconnect.</p>}
                  </Card>

                  <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-3">
                    <CardHeader className="p-0 pb-1 shrink-0 flex flex-row items-center justify-between space-y-0">
                      <CardTitle className="text-xs font-bold text-foreground">App Preferences</CardTitle>
                      <Badge variant="secondary" className="h-4.5 px-1.5 text-[8.5px] font-bold bg-green-500/10 text-green-600 border border-green-500/25 select-none dark:text-green-400">Live</Badge>
                    </CardHeader>
                    <FormSwitchRow title="Close to Tray" desc="Minimize to system tray on close" checked={settings.closeToTray} onChange={(v) => updateSettings({ closeToTray: v })} />
                    <FormSwitchRow title="Launch on System Login" desc="Launch minimized on user login" checked={settings.autostart} onChange={(v) => updateSettings({ autostart: v })} />
                    <FormSwitchRow title="LAN Hotspot Sharing" desc="Bind to 0.0.0.0 for LAN device sharing" checked={settings.wifiSharing} onChange={(v) => updateSettings({ wifiSharing: v })} />
                    <FormSwitchRow
                      title="Node Geolocation Lookup"
                      desc="Share public node IPs with freeipapi.com to resolve country flags (disabled by default; hostnames and credentials are never shared)"
                      checked={settings.geoLookupEnabled}
                      onChange={(enabled) => {
                        updateSettings({ geoLookupEnabled: enabled });
                        if (!enabled) clearNodeGeoCache();
                      }}
                    />
                  </Card>

                  <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-3">
                    <CardHeader className="p-0 pb-1 shrink-0 flex flex-row items-center justify-between space-y-0">
                      <CardTitle className="text-xs font-bold text-foreground">Connection Optimization</CardTitle>
                      <Badge variant="outline" className="h-4.5 px-1.5 text-[8.5px] font-bold text-orange-500 border-orange-500/20 bg-orange-500/5 select-none">Requires restart</Badge>
                    </CardHeader>
                    <FormSwitchRow title="Enable Multiplexing (Mux)" desc="Multiplexes multiple streams over a single connection" checked={settings.multiplex} onChange={(v) => updateSettings({ multiplex: v })} />
                  </Card>

                  <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-3">
                    <CardHeader className="p-0 pb-1 shrink-0 flex flex-row items-center justify-between space-y-0">
                      <CardTitle className="text-xs font-bold text-foreground">Appearance</CardTitle>
                      <Badge variant="secondary" className="h-4.5 px-1.5 text-[8.5px] font-bold bg-green-500/10 text-green-600 border border-green-500/25 select-none dark:text-green-400">Live</Badge>
                    </CardHeader>
                    <SegmentedControl
                      value={theme}
                      onChange={(mode) => setTheme(mode as ThemeMode)}
                      options={[
                        { value: 'light', label: 'Light', icon: 'light_mode' },
                        { value: 'dark', label: 'Dark', icon: 'dark_mode' },
                        { value: 'system', label: 'System', icon: 'desktop_windows' },
                      ]}
                      className="w-full justify-stretch"
                    />
                  </Card>
                </div>

                <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-3 h-fit">
                  <CardHeader className="p-0 pb-1 shrink-0 flex flex-row items-center justify-between space-y-0">
                    <CardTitle className="text-xs font-bold text-foreground">Inbound Ports</CardTitle>
                    <Badge variant="outline" className="h-4.5 px-1.5 text-[8.5px] font-bold text-orange-500 border-orange-500/20 bg-orange-500/5 select-none">Requires restart</Badge>
                  </CardHeader>
                  <FormSwitchRow title="Use Separate HTTP & SOCKS5 Ports" desc="Bind separate HTTP and SOCKS5 protocol listeners"
                    checked={settings.useSeparatePorts} onChange={(v) => updateSettings({ useSeparatePorts: v })} />
                  <div className={!settings.useSeparatePorts ? 'opacity-50' : undefined}>
                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
                      {[
                        { label: 'HTTP Proxy Port', key: 'httpPort' as keyof Settings, val: httpPort, set: (v: number) => setPorts({ httpPort: v }) },
                        { label: 'SOCKS5 Port', key: 'socksPort' as keyof Settings, val: socksPort, set: (v: number) => setPorts({ socksPort: v }) },
                      ].map(({ label, key, val, set }) => {
                        const hasConflict = conflictingPorts.includes(val);
                        const isInvalid = val < 1 || val > 65535;
                        const isUnchecked = !isInvalid && !hasConflict && (portStatuses[val] === undefined || portStatuses[val] === 'unknown');
                        const hasError = hasConflict || isInvalid;
                        return (
                          <div key={key} className="flex flex-col gap-1">
                            <div className="flex justify-between items-center px-0.5">
                              <span className="text-[11px] font-medium text-muted-foreground">{label}</span>
                              <span className={`text-[9px] font-bold ${hasError ? 'text-destructive' : isUnchecked ? 'text-amber-500' : 'text-foreground/60'}`}>
                                {isInvalid ? '● Invalid' : hasConflict ? '● Conflict' : portStatuses[val] === 'own' ? '● X-Link' : portStatuses[val] === 'free' ? '● Free' : '● Unchecked'}
                              </span>
                            </div>
                            <FormInput
                              type="number"
                              aria-label={label}
                              mono
                              className="h-8"
                              style={{ borderColor: hasError ? 'var(--destructive)' : undefined }}
                              value={val}
                              disabled={!settings.useSeparatePorts}
                              onChange={(v) => { const numVal = parseInt(v) || 0; set(numVal); updateSettings({ [key]: numVal } as Partial<Settings>); }}
                            />
                          </div>
                        );
                      })}
                    </div>
                  </div>
                  <div className={`mt-1 ${settings.useSeparatePorts ? 'opacity-50' : ''}`}>
                    <div className="flex flex-col gap-1">
                      <div className="flex justify-between items-center px-0.5">
                        <span className="text-[11px] font-medium text-muted-foreground">Mixed inbound port (recommended)</span>
                        <span className={`text-[9px] font-bold ${conflictingPorts.includes(mixedPort) || mixedPort < 1 || mixedPort > 65535 ? 'text-destructive' : (portStatuses[mixedPort] === undefined || portStatuses[mixedPort] === 'unknown') ? 'text-amber-500' : 'text-foreground/60'}`}>
                          {mixedPort < 1 || mixedPort > 65535 ? '● Invalid' : conflictingPorts.includes(mixedPort) ? '● Conflict' : portStatuses[mixedPort] === 'own' ? '● X-Link' : portStatuses[mixedPort] === 'free' ? '● Free' : '● Unchecked'}
                        </span>
                      </div>
                      <FormInput
                        type="number"
                        aria-label="Mixed inbound port"
                        mono
                        className="h-8"
                        style={{ borderColor: (conflictingPorts.includes(mixedPort) || mixedPort < 1 || mixedPort > 65535) ? 'var(--destructive)' : undefined }}
                        value={mixedPort}
                        disabled={settings.useSeparatePorts}
                        onChange={(v) => { const numVal = parseInt(v) || 0; setPorts({ mixedPort: numVal }); updateSettings({ mixedPort: numVal }); }}
                      />
                    </div>
                  </div>
                  <p className="text-[10px] text-muted-foreground/80 mt-1">Port changes restart the connection when connected.</p>
                  {connectedConfigChange && <p className="text-[10px] text-amber-500">Port and listener changes are applied with a validated restart while connected; on failure the previous values are restored.</p>}
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
                  <Icon name="warning" size={14} className="shrink-0" />
                  <span>TUN Mode requires Administrator elevation. Click "Run as Admin" on the Dashboard.</span>
                </div>
              )}
              <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-4">
                <CardHeader className="p-0 pb-1 shrink-0 flex flex-row items-center justify-between space-y-0">
                  <CardTitle className="text-xs font-bold text-foreground">TUN Interface Configuration</CardTitle>
                  <Badge variant="outline" className="h-4.5 px-1.5 text-[8.5px] font-bold text-orange-500 border-orange-500/20 bg-orange-500/5 select-none">Requires restart</Badge>
                </CardHeader>
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-3.5">
                  <div className="flex flex-col gap-1.5">
                    <FormSelect
                      label="TUN stack implementation" value={settings.tunStack} onChange={(v) => updateSettings({ tunStack: v as Settings['tunStack'] })} options={[
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
                  <span className="text-[10px] text-muted-foreground/80 sm:col-start-2">1400 recommended. Use 1500 for standard ethernet.</span>
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
                <Icon name="info" size={14} className="mt-0.5 text-foreground shrink-0" />
                <span>TUN mode captures ALL system traffic. Combined with Auto Route + Strict Route, it prevents DNS leaks. While connected, TUN interface changes are applied with a validated restart that keeps the current effective mode; on failure the previous values are restored.</span>
              </div>
            </div>
          )}

          {/* ── SNIFFING ── */}
          {settingsTab === 'sniff' && (
            <div className="flex flex-col gap-4">
              <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-4">
                <CardHeader className="p-0 pb-1 shrink-0 flex flex-row items-center justify-between space-y-0">
                  <CardTitle className="text-xs font-bold text-foreground">Traffic Sniffing & Protocol Detection</CardTitle>
                  <Badge variant="outline" className="h-4.5 px-1.5 text-[8.5px] font-bold text-orange-500 border-orange-500/20 bg-orange-500/5 select-none">Requires restart</Badge>
                </CardHeader>
                <FormSwitchRow title="Enable Traffic Sniffing" desc="Inspect connection metadata to extract domain names for routing"
                  checked={settings.sniffEnabled} onChange={(v) => updateSettings({ sniffEnabled: v })} />
                <div className={!settings.sniffEnabled ? 'opacity-50' : undefined}>
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                    {([
                      { key: 'sniffHttp', title: 'HTTP Sniffing', desc: 'Extract hostname from HTTP Host header' },
                      { key: 'sniffTls', title: 'TLS/HTTPS Sniffing', desc: 'Extract SNI from TLS ClientHello' },
                      { key: 'sniffQuic', title: 'QUIC/HTTP3 Sniffing', desc: 'Extract SNI from QUIC Initial packets' },
                      { key: 'sniffOverrideDestination', title: 'Override Destination', desc: 'Use sniffed domain to override target IP' },
                    ] as const).map(({ key, title, desc }) => (
                      <FormSwitchRow key={key} title={title} desc={desc} checked={settings[key]} disabled={!settings.sniffEnabled} onChange={(v) => updateSettings({ [key]: v } as Partial<Settings>)} />
                    ))}
                  </div>
                </div>
              </Card>
              {connectedConfigChange && <p className="text-[10px] text-amber-500">Traffic sniffing changes are applied with a validated restart while connected; on failure the previous values are restored.</p>}
            </div>
          )}

          {/* ── DNS ── */}
          {settingsTab === 'dns' && (
            <div className="flex flex-col gap-4">
              <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-4">
                <CardHeader className="p-0 pb-1 shrink-0 flex flex-row items-center justify-between space-y-0">
                  <CardTitle className="text-xs font-bold text-foreground">DNS Servers</CardTitle>
                  <Badge variant="outline" className="h-4.5 px-1.5 text-[8.5px] font-bold text-orange-500 border-orange-500/20 bg-orange-500/5 select-none">Requires restart</Badge>
                </CardHeader>
                <div className="grid grid-cols-1 sm:grid-cols-3 gap-3.5">
                  <div className="flex flex-col gap-1.5">
                    <FormInput label="Primary DNS (via proxy)" value={settings.primaryDns} onChange={(v) => updateSettings({ primaryDns: v })} placeholder="https://1.1.1.1/dns-query" mono />
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <FormInput label="Fallback DNS (via proxy)" value={settings.fallbackDns} onChange={(v) => updateSettings({ fallbackDns: v })} placeholder="https://8.8.8.8/dns-query" mono />
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <FormInput label="Direct DNS (local/bypass)" value={settings.directDns} onChange={(v) => updateSettings({ directDns: v })} placeholder="Auto (System DNS)" mono />
                  </div>
                </div>
                <p className="text-[10px] text-muted-foreground/80">
                  Used for resolving domains. The direct DNS server is used to resolve direct/bypassed connections.
                </p>
              </Card>

              <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-4">
                <CardHeader className="p-0 pb-1 shrink-0 flex flex-row items-center justify-between space-y-0">
                  <CardTitle className="text-xs font-bold text-foreground">DNS Options & Strategy</CardTitle>
                  <Badge variant="outline" className="h-4.5 px-1.5 text-[8.5px] font-bold text-orange-500 border-orange-500/20 bg-orange-500/5 select-none">Requires restart</Badge>
                </CardHeader>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div className="flex flex-col gap-1.5">
                    <FormSelect
                      label="DNS query strategy" value={settings.dnsStrategy} onChange={(v) => updateSettings({ dnsStrategy: v as Settings['dnsStrategy'] })} options={[
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
                <CardHeader className="p-0 pb-1 shrink-0 flex flex-row items-center justify-between space-y-0">
                  <CardTitle className="text-xs font-bold text-foreground">Resolution Mode</CardTitle>
                  <Badge variant="outline" className="h-4.5 px-1.5 text-[8.5px] font-bold text-orange-500 border-orange-500/20 bg-orange-500/5 select-none">Requires restart</Badge>
                </CardHeader>
                <div className="flex flex-col gap-1.5">
                  <FormSelect
                    label="DNS mode" value={settings.dnsMode} onChange={(v) => updateSettings({ dnsMode: v as DnsMode })} options={[
                    { value: 'fakeip', label: 'FakeIP (recommended for TUN)' },
                    { value: 'normal', label: 'Normal (real IP resolution)' },
                  ]} />
                  <span className="text-[10px] text-muted-foreground/80">
                    FakeIP avoids leaking real destination IPs to the local resolver and speeds up routing decisions.
                  </span>
                </div>
                <div className={settings.dnsMode !== 'fakeip' ? 'opacity-50' : undefined}>
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-3.5">
                    <div className="flex flex-col gap-1.5">
                      <FormInput label="FakeIP range" value={settings.fakeipRange} onChange={(v) => updateSettings({ fakeipRange: v })} placeholder="198.18.0.0/15" disabled={settings.dnsMode !== 'fakeip'} mono />
                    </div>
                    <div className="flex flex-col gap-1.5">
                      <FormInput label="FakeIP exclude filter" value={settings.fakeipFilter} onChange={(v) => updateSettings({ fakeipFilter: v })} placeholder="geosite:private" disabled={settings.dnsMode !== 'fakeip'} mono />
                    </div>
                  </div>
                </div>
              </Card>

              <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-3">
                <CardHeader className="p-0 pb-1 shrink-0 flex flex-row items-center justify-between space-y-0">
                  <CardTitle className="text-xs font-bold text-foreground">Routing</CardTitle>
                  <Badge variant="outline" className="h-4.5 px-1.5 text-[8.5px] font-bold text-orange-500 border-orange-500/20 bg-orange-500/5 select-none">Requires restart</Badge>
                </CardHeader>
                <FormSwitchRow title="Bypass LAN / Private IPs" desc="Route RFC-1918 addresses directly instead of through the proxy"
                  checked={settings.bypassLan} onChange={(v) => updateSettings({ bypassLan: v })} />
              </Card>

              <div className="info-box bg-muted/40 border border-border/40 p-2.5 px-3 rounded-lg flex items-start gap-2 text-[10.5px] text-muted-foreground leading-relaxed shrink-0">
                <Icon name="info" size={14} className="mt-0.5 text-foreground shrink-0" />
                <span>While connected, DNS changes are applied with a validated restart; on failure the previous values are restored.</span>
              </div>
            </div>
          )}

          {/* ── API ── */}
          {settingsTab === 'api' && (
            <div className="flex flex-col gap-4">
              <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-4">
                <CardHeader className="p-0 pb-1 shrink-0 flex flex-row items-center justify-between space-y-0">
                  <CardTitle className="text-xs font-bold text-foreground">Clash-Compatible REST API</CardTitle>
                  <Badge variant="outline" className="h-4.5 px-1.5 text-[8.5px] font-bold text-orange-500 border-orange-500/20 bg-orange-500/5 select-none">Requires restart</Badge>
                </CardHeader>
                <FormSwitchRow title="Enable External API" desc="Expose HTTP API for external controllers (Yacd, MetaCubeX)"
                  checked={settings.apiEnabled} onChange={(v) => updateSettings({ apiEnabled: v })} />
                <div className={`flex flex-col gap-4 ${!settings.apiEnabled ? 'opacity-50' : ''}`}>
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-3.5">
                    <FormInput
                      label="API Port"
                      type="number"
                      mono
                      className="h-8"
                      style={{ borderColor: (settings.apiPort < 1 || settings.apiPort > 65535) ? 'var(--destructive)' : undefined }}
                      value={settings.apiPort}
                      disabled={!settings.apiEnabled}
                      onChange={(v) => updateSettings({ apiPort: parseInt(v) || 9090 })}
                    />
                    <div className="flex flex-col gap-1.5">
                      <div className="flex gap-2 items-center">
                        <div className="flex-1 min-w-0">
                          <FormInput label="API secret token" value={settings.apiSecret} onChange={(v) => updateSettings({ apiSecret: v })} placeholder="Leave empty for no auth" disabled={!settings.apiEnabled} mono />
                        </div>
                        <Button
                          type="button"
                          variant="outline"
                          size="icon"
                          className="size-8 shrink-0"
                          title="Generate new random API Secret Token"
                          disabled={!settings.apiEnabled}
                          onClick={() => {
                            const chars = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
                            const secret = Array.from({ length: 16 }, () => {
                              const random = new Uint32Array(1);
                              do { crypto.getRandomValues(random); } while (random[0] >= Math.floor(2 ** 32 / chars.length) * chars.length);
                              return chars[random[0] % chars.length];
                            }).join('');
                            updateSettings({ apiSecret: secret });
                          }}
                        >
                          <Icon name="refresh" size={14} />
                        </Button>
                      </div>
                    </div>
                  </div>
                  <FormSwitchRow title="Allow CORS (Cross-Origin)" desc="Allow web-based controllers (Yacd) to connect from browser"
                    checked={settings.apiCors} disabled={!settings.apiEnabled} onChange={(v) => updateSettings({ apiCors: v })} />
                  {settings.apiEnabled && isConnected && (
                    <div className="bg-muted/80 text-foreground text-[11px] px-3.5 py-2.5 rounded border border-border/50 flex items-center justify-between gap-2">
                      <div className="flex items-center gap-2">
                        <Icon name="check_circle" size={14} className="text-muted-foreground shrink-0" />
                        <span className="leading-normal">
                          API active at <code className="font-mono text-[10.5px] bg-background border border-border px-1 py-0.5 rounded text-foreground">
                            http://127.0.0.1:{settings.apiPort}
                          </code>
                          {' — open in '}
                          <a
                            href={`https://yacd.haishan.me/?hostname=127.0.0.1&port=${settings.apiPort}`}
                            target="_blank"
                            rel="noreferrer noopener"
                            className="text-foreground font-semibold underline underline-offset-2 inline-flex items-center gap-0.5"
                          >
                            Yacd UI <Icon name="open_in_new" size={12} className="inline-block" />
                          </a>
                        </span>
                      </div>
                      {settings.apiSecret && (
                        <Button
                          type="button"
                          variant="outline"
                          size="sm"
                          className="h-7 text-xs font-semibold gap-1.5 shrink-0"
                          title="Copy API secret token to clipboard for dashboard login"
                          onClick={async () => {
                            try {
                              await navigator.clipboard.writeText(settings.apiSecret);
                              useToastStore.getState().addToast('success', 'API secret copied to clipboard for dashboard login.', 'Secret Copied');
                            } catch {
                              useToastStore.getState().addToast('error', 'Failed to copy secret to clipboard.', 'Copy Failed');
                            }
                          }}
                        >
                          <Icon name="content_copy" size={12} />
                          Copy Token
                        </Button>
                      )}
                    </div>
                  )}
                </div>
              </Card>
              <div className="info-box bg-muted/40 border border-border/40 p-2.5 px-3 rounded-lg flex items-start gap-2 text-[10.5px] text-muted-foreground leading-relaxed shrink-0">
                <Icon name="info" size={14} className="mt-0.5 text-foreground shrink-0" />
                <span>The Clash-compatible API lets Yacd or MetaCubeX dashboard control routing and monitor connections in real-time.</span>
              </div>
            </div>
          )}
        </div>
      </div>
    </ViewShell>
  );
}
