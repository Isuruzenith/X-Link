import React from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Icon } from '@/components/ui/icon';
import { ViewShell } from '../components/ViewShell';
import { useSettingsStore, type ThemeMode } from '../stores/settingsStore';
import { useConnectionStore } from '../stores/connectionStore';
import { useToastStore } from '../stores/toastStore';
import { useProfileStore } from '../stores/profileStore';
import type { Settings, DnsMode } from '../utils/store';
import { generateApiSecret } from '../utils/store';
import xLinkLogo from '../assets/X-Link-logo.png';
import { Card, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { FormInput, FormSelect, FormSwitchRow, SegmentedControl } from '@/components/form';

type SettingsSection = 'general' | 'tun' | 'sniff' | 'dns' | 'api' | 'diagnostics';

export interface RoutingSimulationResult {
  destination: string;
  destinationType: 'ip' | 'domain';
  matchedOutbound: string;
  matchedRuleId: string | null;
  matchedRuleType: string | null;
  matchReason: string;
  isLanBypass: boolean;
  dnsStrategy: string;
}

export interface DiagnosticsReport {
  timestamp: number;
  appVersion: string;
  os: { platform: string; arch: string; isElevated: boolean };
  sidecar: {
    bundledTestedVersion: string;
    cachedVersion: string | null;
    isCompatible: boolean;
    compatibilityError: string | null;
  };
  proxy: { status: string; effectiveMode: string | null; tunSupported: boolean };
  ports: {
    mixedPort: number;
    socksPort: number;
    httpPort: number;
    apiPort: number;
    portStatuses: Record<number, string>;
  };
  systemProxy: { managedByXlink: boolean };
  profiles: {
    profileCount: number;
    activeProfileType: string | null;
    nodeCountsByType: Record<string, number>;
  };
  settings: Record<string, unknown>;
  recentLogs: string[];
}

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
  const [simDestination, setSimDestination] = React.useState('');
  const [simResult, setSimResult] = React.useState<RoutingSimulationResult | null>(null);
  const [isSimulating, setIsSimulating] = React.useState(false);
  const [diagReport, setDiagReport] = React.useState<DiagnosticsReport | null>(null);
  const [isExportingDiag, setIsExportingDiag] = React.useState(false);
  const [showDiagPreview, setShowDiagPreview] = React.useState(false);

  const handleSimulate = async () => {
    const dest = simDestination.trim();
    if (!dest) return;
    setIsSimulating(true);
    try {
      const res = await invoke<RoutingSimulationResult>('simulate_routing', { destination: dest });
      setSimResult(res);
    } catch (err) {
      useToastStore.getState().addToast('error', `Simulation failed: ${err}`, 'Simulation Error');
    } finally {
      setIsSimulating(false);
    }
  };

  const handleFetchDiagnostics = async () => {
    setIsExportingDiag(true);
    try {
      const report = await invoke<DiagnosticsReport>('export_system_diagnostics');
      setDiagReport(report);
      return report;
    } catch (err) {
      useToastStore.getState().addToast('error', `Failed to export diagnostics: ${err}`, 'Diagnostics Error');
      return null;
    } finally {
      setIsExportingDiag(false);
    }
  };

  const handleCopyDiagnostics = async () => {
    const report = diagReport || (await handleFetchDiagnostics());
    if (report) {
      try {
        await navigator.clipboard.writeText(JSON.stringify(report, null, 2));
        useToastStore.getState().addToast('success', 'Sanitized diagnostics copied to clipboard.', 'Copied');
      } catch {
        useToastStore.getState().addToast('error', 'Clipboard write failed.', 'Error');
      }
    }
  };

  const handleDownloadDiagnostics = async () => {
    const report = diagReport || (await handleFetchDiagnostics());
    if (report) {
      try {
        const blob = new Blob([JSON.stringify(report, null, 2)], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `xlink-diagnostics-${Date.now()}.json`;
        a.click();
        URL.revokeObjectURL(url);
        useToastStore.getState().addToast('success', 'Diagnostics file downloaded.', 'Downloaded');
      } catch {
        useToastStore.getState().addToast('error', 'Download failed.', 'Error');
      }
    }
  };

  const sideNavItems: [SettingsSection, string, string][] = [
    ['general', 'tune', 'General'],
    ['tun', 'router', 'TUN Mode'],
    ['sniff', 'visibility', 'Sniffing'],
    ['dns', 'dns', 'DNS'],
    ['api', 'code', 'API'],
    ['diagnostics', 'troubleshoot', 'Diagnostics'],
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
                      className="appearance-segments"
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

              <Card className="settings-version-card">
                <img src={xLinkLogo} alt="X-Link" />
                <div className="version-copy">
                  <h4>X-Link</h4>
                  <p>
                    Empowered by sing-box
                  </p>
                </div>
                <div className="version-badge">
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
                    <div className="flex flex-wrap gap-1 mt-0.5">
                      {[
                        { label: 'Cloudflare DoH', val: 'https://1.1.1.1/dns-query' },
                        { label: 'Google DoH', val: 'https://dns.google/dns-query' },
                        { label: 'Cloudflare DoT', val: 'tls://1.1.1.1:853' },
                        { label: 'AdGuard DoQ', val: 'quic://dns.adguard-dns.com:853' },
                      ].map((item) => (
                        <button
                          key={item.label}
                          type="button"
                          className="text-[9.5px] px-1.5 py-0.5 rounded bg-muted hover:bg-muted/80 text-muted-foreground hover:text-foreground cursor-pointer transition-colors"
                          onClick={() => updateSettings({ primaryDns: item.val })}
                        >
                          {item.label}
                        </button>
                      ))}
                    </div>
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <FormInput label="Fallback DNS (via proxy)" value={settings.fallbackDns} onChange={(v) => updateSettings({ fallbackDns: v })} placeholder="https://8.8.8.8/dns-query" mono />
                    <div className="flex flex-wrap gap-1 mt-0.5">
                      {[
                        { label: 'Google DoH', val: 'https://dns.google/dns-query' },
                        { label: 'Quad9 DoH', val: 'https://dns.quad9.net/dns-query' },
                        { label: 'Google DoT', val: 'tls://8.8.8.8:853' },
                        { label: 'Cloudflare UDP', val: '1.1.1.1:53' },
                      ].map((item) => (
                        <button
                          key={item.label}
                          type="button"
                          className="text-[9.5px] px-1.5 py-0.5 rounded bg-muted hover:bg-muted/80 text-muted-foreground hover:text-foreground cursor-pointer transition-colors"
                          onClick={() => updateSettings({ fallbackDns: item.val })}
                        >
                          {item.label}
                        </button>
                      ))}
                    </div>
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <FormInput label="Direct DNS (local/bypass)" value={settings.directDns} onChange={(v) => updateSettings({ directDns: v })} placeholder="Auto (System DNS)" mono />
                    <div className="flex flex-wrap gap-1 mt-0.5">
                      {[
                        { label: 'System (Auto)', val: '' },
                        { label: 'Cloudflare', val: '1.1.1.1:53' },
                        { label: 'Google', val: '8.8.8.8:53' },
                      ].map((item) => (
                        <button
                          key={item.label}
                          type="button"
                          className="text-[9.5px] px-1.5 py-0.5 rounded bg-muted hover:bg-muted/80 text-muted-foreground hover:text-foreground cursor-pointer transition-colors"
                          onClick={() => updateSettings({ directDns: item.val })}
                        >
                          {item.label}
                        </button>
                      ))}
                    </div>
                  </div>
                </div>
                <p className="text-[10px] text-muted-foreground/80">
                  Supports DoH (<code>https://</code>), DoT (<code>tls://</code>), DoQ (<code>quic://</code>), TCP (<code>tcp://</code>), and Traditional UDP (<code>ip:port</code>).
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
                  checked={settings.apiEnabled} onChange={(v) => {
                    if (v && !settings.apiSecret.trim()) {
                      const secret = generateApiSecret();
                      updateSettings({ apiEnabled: v, apiSecret: secret });
                    } else {
                      updateSettings({ apiEnabled: v });
                    }
                  }} />
                <div className={`flex flex-col gap-4 ${!settings.apiEnabled ? 'opacity-50' : ''}`}>
                  <div className="settings-api-row">
                    <div className="settings-api-port">
                      <FormInput
                        label="API Port"
                        type="number"
                        mono
                        style={{ borderColor: (settings.apiPort < 1 || settings.apiPort > 65535) ? 'var(--destructive)' : undefined }}
                        value={settings.apiPort}
                        disabled={!settings.apiEnabled}
                        onChange={(v) => updateSettings({ apiPort: parseInt(v) || 9090 })}
                      />
                    </div>
                    <div className="settings-api-secret">
                      <div className="settings-api-secret-action">
                        <FormInput
                          label="API secret token"
                          value={settings.apiSecret}
                          onChange={(v) => updateSettings({ apiSecret: v })}
                          placeholder="Secret token required"
                          disabled={!settings.apiEnabled}
                          mono
                          style={{ borderColor: settings.apiEnabled && !settings.apiSecret.trim() ? 'var(--destructive)' : undefined }}
                        />
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          className="settings-api-generate"
                          title="Generate new random API Secret Token"
                          disabled={!settings.apiEnabled}
                          onClick={() => {
                            const secret = generateApiSecret();
                            updateSettings({ apiSecret: secret });
                          }}
                        >
                          <Icon name="refresh" size={14} />
                        </Button>
                      </div>
                    </div>
                  </div>
                  {settings.apiEnabled && !settings.apiSecret.trim() && (
                    <div className="text-[11px] text-destructive flex items-center gap-1.5 -mt-2">
                      <Icon name="warning" size={14} className="shrink-0" />
                      <span>API secret token is required. Controller will not start without authentication.</span>
                    </div>
                  )}
                  <FormSwitchRow title="Allow CORS (Cross-Origin)" desc="Allow web-based controllers (Yacd) to connect from browser"
                    checked={settings.apiCors} disabled={!settings.apiEnabled} onChange={(v) => updateSettings({ apiCors: v })} />
                  {settings.apiEnabled && isConnected && (
                    settings.apiSecret.trim() ? (
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
                            {!settings.apiCors && (
                              <span className="text-muted-foreground text-[10px] ml-1">
                                (Requires "Allow CORS" for web dashboard)
                              </span>
                            )}
                          </span>
                        </div>
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
                      </div>
                    ) : (
                      <div className="bg-destructive/10 text-destructive text-[11px] px-3.5 py-2.5 rounded border border-destructive/20 flex items-center gap-2">
                        <Icon name="warning" size={14} className="shrink-0" />
                        <span>API controller is disabled because secret token is missing. Please set or generate a token and restart.</span>
                      </div>
                    )
                  )}
                </div>
              </Card>
              <div className="info-box bg-muted/40 border border-border/40 p-2.5 px-3 rounded-lg flex items-start gap-2 text-[10.5px] text-muted-foreground leading-relaxed shrink-0">
                <Icon name="info" size={14} className="mt-0.5 text-foreground shrink-0" />
                <span>The Clash-compatible API lets Yacd or MetaCubeX dashboard control routing and monitor connections in real-time.</span>
              </div>
            </div>
          )}

          {/* ── DIAGNOSTICS ── */}
          {settingsTab === 'diagnostics' && (
            <div className="flex flex-col gap-4">
              {/* System Health Overview Card */}
              <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-3">
                <CardHeader className="p-0 pb-1 shrink-0 flex flex-row items-center justify-between space-y-0">
                  <CardTitle className="text-xs font-bold text-foreground">System Health & Bundle</CardTitle>
                  <Badge variant="outline" className="h-4.5 px-1.5 text-[8.5px] font-bold text-primary border-primary/20 bg-primary/5 select-none">Privacy Safe</Badge>
                </CardHeader>
                <p className="text-[11px] text-muted-foreground leading-relaxed">
                  Export a sanitized diagnostic snapshot of core system services, port bindings, sing-box sidecar health, and redacted log excerpts. Sensitive tokens and passwords are scrubbed before export.
                </p>

                <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 pt-1">
                  <div className="bg-muted/40 p-2.5 rounded-lg border border-border/40 flex flex-col gap-0.5">
                    <span className="text-[10px] text-muted-foreground font-medium uppercase tracking-wider">Elevation</span>
                    <span className="text-xs font-semibold flex items-center gap-1.5">
                      <Icon name={isElevated ? 'verified_user' : 'person'} size={14} className={isElevated ? 'text-primary' : 'text-muted-foreground'} />
                      {isElevated ? 'Administrator' : 'Standard User'}
                    </span>
                  </div>
                  <div className="bg-muted/40 p-2.5 rounded-lg border border-border/40 flex flex-col gap-0.5">
                    <span className="text-[10px] text-muted-foreground font-medium uppercase tracking-wider">Proxy Core</span>
                    <span className="text-xs font-semibold flex items-center gap-1.5">
                      <Icon name={isConnected ? 'check_circle' : 'stop_circle'} size={14} className={isConnected ? 'text-green-500' : 'text-muted-foreground'} />
                      {isConnected ? 'Running' : 'Stopped'}
                    </span>
                  </div>
                  <div className="bg-muted/40 p-2.5 rounded-lg border border-border/40 flex flex-col gap-0.5">
                    <span className="text-[10px] text-muted-foreground font-medium uppercase tracking-wider">Current Mode</span>
                    <span className="text-xs font-semibold uppercase font-mono">
                      {settings.proxyMode}
                    </span>
                  </div>
                  <div className="bg-muted/40 p-2.5 rounded-lg border border-border/40 flex flex-col gap-0.5">
                    <span className="text-[10px] text-muted-foreground font-medium uppercase tracking-wider">Port Conflicts</span>
                    <span className="text-xs font-semibold flex items-center gap-1.5">
                      <Icon name={conflictingPorts.length === 0 ? 'check' : 'warning'} size={14} className={conflictingPorts.length === 0 ? 'text-green-500' : 'text-amber-500'} />
                      {conflictingPorts.length === 0 ? 'None detected' : `${conflictingPorts.length} conflict(s)`}
                    </span>
                  </div>
                </div>

                <div className="flex flex-wrap items-center gap-2 pt-2">
                  <Button
                    type="button"
                    variant="default"
                    size="sm"
                    className="h-8 text-xs font-semibold gap-1.5"
                    disabled={isExportingDiag}
                    onClick={async () => {
                      await handleFetchDiagnostics();
                      setShowDiagPreview(true);
                    }}
                  >
                    <Icon name="assignment" size={14} />
                    {isExportingDiag ? 'Generating...' : 'Generate Diagnostics'}
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="h-8 text-xs font-semibold gap-1.5"
                    onClick={handleCopyDiagnostics}
                    disabled={isExportingDiag}
                  >
                    <Icon name="content_copy" size={14} />
                    Copy JSON
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="h-8 text-xs font-semibold gap-1.5"
                    onClick={handleDownloadDiagnostics}
                    disabled={isExportingDiag}
                  >
                    <Icon name="download" size={14} />
                    Download Report
                  </Button>
                  {diagReport && (
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      className="h-8 text-xs gap-1 ml-auto text-muted-foreground"
                      onClick={() => setShowDiagPreview((prev) => !prev)}
                    >
                      <Icon name={showDiagPreview ? 'visibility_off' : 'visibility'} size={14} />
                      {showDiagPreview ? 'Hide Preview' : 'Show Preview'}
                    </Button>
                  )}
                </div>

                {showDiagPreview && diagReport && (
                  <div className="mt-2 flex flex-col gap-1.5">
                    <div className="flex items-center justify-between text-[11px] text-muted-foreground">
                      <span>Diagnostics Preview ({new Date(diagReport.timestamp * 1000).toLocaleTimeString()})</span>
                      <span className="text-green-500 font-medium">✓ Sanitized</span>
                    </div>
                    <pre className="p-3 bg-muted/60 rounded-md border border-border/60 text-[10.5px] font-mono overflow-auto max-h-60 text-foreground/90 whitespace-pre">
                      {JSON.stringify(diagReport, null, 2)}
                    </pre>
                  </div>
                )}
              </Card>

              {/* Route & DNS Simulation Explainer */}
              <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-3">
                <CardHeader className="p-0 pb-1 shrink-0 flex flex-row items-center justify-between space-y-0">
                  <CardTitle className="text-xs font-bold text-foreground">Routing Rule & DNS Simulation</CardTitle>
                  <Badge variant="secondary" className="h-4.5 px-1.5 text-[8.5px] font-bold bg-primary/10 text-primary border border-primary/25 select-none">Simulate</Badge>
                </CardHeader>
                <p className="text-[11px] text-muted-foreground leading-relaxed">
                  Test how destinations (domains, IP addresses, CIDR blocks) resolve through X-Link routing logic, LAN bypass rules, and active outbounds.
                </p>

                <div className="flex flex-col sm:flex-row gap-2 pt-1">
                  <div className="flex-1">
                    <FormInput
                      label="Destination (Domain or IP)"
                      placeholder="e.g. 192.168.1.1, google.com, or 10.0.0.5"
                      value={simDestination}
                      onChange={(val) => setSimDestination(val)}
                      mono
                    />
                  </div>
                  <div className="flex items-end">
                    <Button
                      type="button"
                      variant="default"
                      size="sm"
                      className="h-9 px-4 text-xs font-semibold gap-1.5 shrink-0"
                      disabled={!simDestination.trim() || isSimulating}
                      onClick={handleSimulate}
                    >
                      <Icon name="play_arrow" size={14} />
                      {isSimulating ? 'Evaluating...' : 'Simulate'}
                    </Button>
                  </div>
                </div>

                {simResult && (
                  <div className="mt-2 p-3 bg-muted/30 border border-border/50 rounded-lg flex flex-col gap-2.5">
                    <div className="flex items-center justify-between flex-wrap gap-2">
                      <div className="flex items-center gap-2">
                        <span className="text-xs font-semibold text-foreground">{simResult.destination}</span>
                        <Badge variant="outline" className="text-[9.5px] uppercase font-mono px-1.5 py-0">
                          {simResult.destinationType}
                        </Badge>
                      </div>
                      <div className="flex items-center gap-1.5">
                        <span className="text-[11px] text-muted-foreground">Target Outbound:</span>
                        <Badge
                          variant={
                            simResult.matchedOutbound === 'direct'
                              ? 'secondary'
                              : simResult.matchedOutbound === 'block'
                              ? 'destructive'
                              : 'default'
                          }
                          className="text-xs font-bold px-2 py-0.5"
                        >
                          {simResult.matchedOutbound.toUpperCase()}
                        </Badge>
                      </div>
                    </div>

                    <div className="grid grid-cols-1 sm:grid-cols-3 gap-2 pt-1 text-[11px]">
                      <div className="p-2 bg-background/50 rounded border border-border/30">
                        <span className="text-muted-foreground text-[10px] block">Match Reason</span>
                        <span className="font-medium text-foreground">{simResult.matchReason}</span>
                      </div>
                      <div className="p-2 bg-background/50 rounded border border-border/30">
                        <span className="text-muted-foreground text-[10px] block">Rule Matched</span>
                        <span className="font-mono text-foreground">
                          {simResult.matchedRuleId ? `${simResult.matchedRuleType}: ${simResult.matchedRuleId}` : 'Default Fallback'}
                        </span>
                      </div>
                      <div className="p-2 bg-background/50 rounded border border-border/30">
                        <span className="text-muted-foreground text-[10px] block">Network Scope / DNS</span>
                        <span className="font-medium text-foreground">
                          {simResult.isLanBypass ? 'LAN / Direct Bypass' : 'Public Internet'} ({simResult.dnsStrategy})
                        </span>
                      </div>
                    </div>
                  </div>
                )}
              </Card>

              <div className="info-box bg-muted/40 border border-border/40 p-2.5 px-3 rounded-lg flex items-start gap-2 text-[10.5px] text-muted-foreground leading-relaxed shrink-0">
                <Icon name="info" size={14} className="mt-0.5 text-foreground shrink-0" />
                <span>Diagnostics help troubleshoot routing loops, port collisions, and DNS resolution issues without revealing personal credentials.</span>
              </div>
            </div>
          )}
        </div>
      </div>
    </ViewShell>
  );
}
