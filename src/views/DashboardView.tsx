import { useEffect } from 'react';
import {
  DownloadCloud, UploadCloud, Clock, Server, Power,
  Shield, ShieldAlert, Network, Wifi, Zap
} from 'lucide-react';
import { TrafficChart } from '../components/TrafficChart';
import { ViewShell } from '../components/ViewShell';
import { useConnectionStore } from '../stores/connectionStore';
import { useSettingsStore } from '../stores/settingsStore';
import { useProfileStore, getCountryCode } from '../stores/profileStore';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';

const formatBytes = (bytes: number): string => {
  if (bytes === 0) return '0 B';
  const k = 1024, sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
};

const formatSpeed = (bps: number): string => {
  if (bps === 0) return '0 B/s';
  const k = 1024, sizes = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
  const i = Math.floor(Math.log(bps) / Math.log(k));
  return parseFloat((bps / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
};

const formatUptime = (seconds: number): string => {
  const h = Math.floor(seconds / 3600).toString().padStart(2, '0');
  const m = Math.floor((seconds % 3600) / 60).toString().padStart(2, '0');
  const s = (seconds % 60).toString().padStart(2, '0');
  return `${h}h ${m}m ${s}s`;
};

interface DashboardViewProps {
  onNavigateToTab?: (tab: 'connections') => void;
}

export function DashboardView({ onNavigateToTab }: DashboardViewProps) {
  const {
    isConnected,
    connectionStatus,
    uptime,
    uploadBytes,
    downloadBytes,
    uploadSpeed,
    downloadSpeed,
    activeConnections,
    speedHistory,
    toggleConnect,
    requestElevation,
  } = useConnectionStore();

  const { settings, isElevated } = useSettingsStore();
  const activeProfile = useProfileStore((s) => s.activeProfile)();
  const {
    nodes,
    selectedNodeTag,
    selectNode,
    selectProfile,
    nodeGeoCache,
    fetchNodeGeo,
  } = useProfileStore();

  const activeProfileId = activeProfile?.id;
  useEffect(() => {
    if (activeProfileId) {
      selectProfile(activeProfileId);
    }
  }, [activeProfileId, selectProfile]);

  const activeNode = selectedNodeTag ? nodes.find((n) => n.tag === selectedNodeTag) : null;

  useEffect(() => {
    if (isConnected && activeNode && activeNode.server && !nodeGeoCache[activeNode.server]) {
      fetchNodeGeo(activeNode.server, activeNode.tag);
    }
  }, [isConnected, activeNode, nodeGeoCache, fetchNodeGeo]);

  // Clear connecting animation immediately on status change
  useEffect(() => {
    const el = document.querySelector('.power-button-outer');
    if (el) {
      if (connectionStatus !== 'connecting') {
        el.classList.remove('connecting');
      }
    }
  }, [connectionStatus]);

  return (
    <ViewShell
      title="Dashboard"
      subtitle="Real-time status monitor and proxy controls"
      actions={
        <div className="flex items-center gap-2">
          {isElevated ? (
            <Badge variant="outline" className="flex items-center justify-center gap-1.5 px-2.5 py-1 text-xs border-border bg-muted/40 text-foreground leading-none">
              <Shield className="size-3.5 text-muted-foreground" />
              <span>TUN: Active</span>
            </Badge>
          ) : (
            <Button variant="destructive" size="sm" className="h-7 gap-1 px-2.5 text-xs font-semibold" onClick={requestElevation}>
              <ShieldAlert className="size-3.5" />
              <span>TUN: Elevation Required</span>
            </Button>
          )}
        </div>
      }
    >
      <div className="flex flex-col flex-1 min-h-0 w-full gap-2">
        {/* Top row: Connect + Chart */}
        <div className="grid grid-cols-1 md:grid-cols-3 flex-1 w-full gap-2">
          {/* Connect Panel */}
          <Card className="flex flex-col p-3.5 bg-card border border-border shadow-sm h-full overflow-hidden min-h-[280px] min-w-0">
            <div className="flex flex-col items-center justify-center flex-1 min-h-0 py-2">
              <div className="power-button-container shrink-0">
                <div className={`power-button-outer ${connectionStatus === 'connected' ? 'connected' : connectionStatus === 'connecting' ? 'connecting' : ''}`} />
                <button
                  type="button"
                  aria-label={connectionStatus === 'connected' ? 'Disconnect' : 'Connect'}
                  aria-pressed={connectionStatus === 'connected'}
                  className={`power-button ${connectionStatus === 'connected' ? 'connected' : connectionStatus === 'connecting' ? 'connecting' : ''}`}
                  onClick={toggleConnect}
                >
                  <Power size={26} />
                </button>
              </div>
              <div className="flex flex-col items-center gap-1 mt-3.5 text-center">
                <h3 className="connect-status-label font-bold text-sm tracking-wide text-foreground">
                  {connectionStatus === 'connected' ? 'Connected' : connectionStatus === 'connecting' ? 'Connecting...' : 'Disconnected'}
                </h3>
                <p className="connect-status-sub text-xs text-muted-foreground max-w-[200px] truncate leading-normal">
                  {connectionStatus === 'connected'
                    ? `${activeProfile?.name || 'Default'}`
                    : connectionStatus === 'connecting'
                      ? 'Establishing secure tunnels...'
                      : 'Toggle power to start proxy'}
                </p>
              </div>
              {isConnected && (
                <Badge variant="secondary" className="connect-mode-badge mt-2.5 flex items-center justify-center gap-1 text-2xs font-medium border border-border/40 px-2 py-0.5 leading-none">
                  <Network className="size-2.5" />
                  <span>{settings.proxyMode === 'tun' ? 'TUN Mode' : 'System Proxy'}</span>
                </Badge>
              )}
            </div>

            {/* Active Node & Quick Switch details */}
            {isConnected && (selectedNodeTag || nodes.length > 1) && (
              <div className="w-full flex flex-col gap-3.5 shrink-0 mt-auto pt-3 border-t border-border/10">
                {/* Active Node */}
                {selectedNodeTag && (
                  <div
                    className="w-full bg-muted/20 border border-border/40 rounded-lg p-3 flex flex-col items-center justify-center text-center gap-2"
                    title={activeNode ? `${selectedNodeTag} (${activeNode.server}:${activeNode.port})` : selectedNodeTag}
                  >
                    {(() => {
                      const geo = activeNode ? nodeGeoCache[activeNode.server] : null;
                      const activeServerCode = geo && geo !== 'loading'
                        ? geo.countryCode
                        : (activeNode ? getCountryCode(activeNode.tag) : null);
                      const activeRegion = geo && geo !== 'loading' && (geo.cityName || geo.regionName)
                        ? `${geo.cityName || ''}${geo.cityName && geo.regionName ? ', ' : ''}${geo.regionName || ''}`
                        : '';
                      const activeCountryName = geo && geo !== 'loading' ? geo.countryName : '';

                      return (
                        <>
                          {activeServerCode ? (
                            <div className="flex items-center justify-center shrink-0 w-5.5 h-4 overflow-hidden rounded-[2px] border border-border/40 shadow-sm">
                              <img
                                src={`https://flagcdn.com/w40/${activeServerCode.toLowerCase()}.png`}
                                alt={activeServerCode}
                                className="w-full h-full object-cover"
                                onError={(e) => {
                                  (e.target as HTMLImageElement).style.display = 'none';
                                }}
                              />
                            </div>
                          ) : (
                            <Zap className="size-3.5 text-muted-foreground shrink-0" />
                          )}
                          <div className="flex flex-col items-center min-w-0 w-full">
                            <span className="text-xs font-bold text-foreground truncate max-w-full flex items-center gap-1 justify-center" title={selectedNodeTag}>
                              <Zap className="size-2.5 text-foreground shrink-0 animate-pulse" />
                              {selectedNodeTag}
                            </span>
                            {(activeCountryName || activeRegion) && (
                              <span className="text-2xs text-muted-foreground truncate max-w-full mt-0.5" title={`${activeCountryName}${activeCountryName && activeRegion ? ' - ' : ''}${activeRegion}`}>
                                {activeCountryName}{activeCountryName && activeRegion ? ' - ' : ''}{activeRegion}
                              </span>
                            )}
                          </div>
                        </>
                      );
                    })()}
                  </div>
                )}

                {/* Quick Switch */}
                {nodes.length > 1 && (
                  <div className="w-full flex flex-col">
                    <span className="text-[9px] font-bold tracking-wider text-muted-foreground uppercase mb-2 text-center block w-full">Quick Switch</span>
                    <ScrollArea className="max-h-16 w-full">
                      <div className="flex flex-wrap gap-1.5 justify-center pr-0">
                        {nodes.slice(0, 8).map((node) => (
                          <Button
                            key={node.tag}
                            onClick={() => selectNode(node)}
                            variant={selectedNodeTag === node.tag ? 'default' : 'secondary'}
                            size="xs"
                            className="h-6 px-2.5 text-2xs font-semibold rounded-full shrink-0 whitespace-nowrap"
                            title={node.tag}
                          >
                            <span className="truncate max-w-[100px]">{node.tag}</span>
                          </Button>
                        ))}
                        {nodes.length > 8 && (
                          <span className="text-2xs text-muted-foreground px-1.5 py-0.5 font-semibold shrink-0 self-center">+{nodes.length - 8}</span>
                        )}
                      </div>
                    </ScrollArea>
                  </div>
                )}
              </div>
            )}
          </Card>

          {/* Chart */}
          <Card className="col-span-1 md:col-span-2 bg-card border border-border shadow-sm flex flex-col overflow-hidden h-full min-h-[280px] p-3.5 min-w-0">
            <CardHeader className="p-0 pb-3 flex flex-row items-center justify-between space-y-0 shrink-0">
              <CardTitle className="text-sm font-semibold tracking-tight">Bandwidth</CardTitle>
              <div className="flex items-center gap-4 text-xs shrink-0 min-w-0">
                <div className="flex items-center gap-1.5 text-muted-foreground min-w-0">
                  <div className="size-1.5 rounded-full bg-foreground shrink-0" />
                  <span className="font-mono text-xs tabular-nums truncate max-w-[90px] sm:max-w-[120px]" title={formatSpeed(downloadSpeed)}>↓ {formatSpeed(downloadSpeed)}</span>
                </div>
                <div className="flex items-center gap-1.5 text-muted-foreground min-w-0">
                  <div className="size-1.5 rounded-full bg-muted-foreground border border-border shrink-0" />
                  <span className="font-mono text-xs tabular-nums truncate max-w-[90px] sm:max-w-[120px]" title={formatSpeed(uploadSpeed)}>↑ {formatSpeed(uploadSpeed)}</span>
                </div>
              </div>
            </CardHeader>
            <CardContent className="p-0 flex-1 relative min-h-[180px] overflow-hidden">
              <TrafficChart history={speedHistory} />
            </CardContent>
          </Card>
        </div>

        {/* Metrics */}
        <div className="flex flex-wrap shrink-0 w-full gap-2">
          {[
            { icon: DownloadCloud, label: 'Total Down',   value: formatBytes(downloadBytes) },
            { icon: UploadCloud,   label: 'Total Up',     value: formatBytes(uploadBytes) },
            { icon: Clock,         label: 'Uptime',       value: formatUptime(uptime) },
            { icon: Server,        label: 'Connections',  value: `${activeConnections} active`, clickable: true },
          ].map(({ icon: Icon, label, value, clickable }) => (
            <Card
              key={label}
              className={`p-3.5 bg-card border border-border shadow-sm flex flex-row items-center gap-3 transition-colors flex-1 min-w-[140px] shrink-0 ${clickable ? 'cursor-pointer hover:bg-accent/40 active:bg-accent/70' : ''}`}
              onClick={clickable ? () => onNavigateToTab?.('connections') : undefined}
            >
              <div className="size-9 rounded-md bg-muted flex items-center justify-center text-foreground border border-border/40 shrink-0">
                <Icon className="size-5" />
              </div>
              <div className="flex flex-col min-w-0 flex-1">
                <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider mb-0.5">{label}</span>
                <span className="text-base font-bold text-foreground truncate tabular-nums leading-none" title={value}>{value}</span>
              </div>
            </Card>
          ))}
        </div>

        {/* Ports bar */}
        <div 
          className="bg-card border-t border-border shadow-sm shrink-0 rounded-none py-2 px-4 -mx-4 -mb-4 mt-2"
        >
          <div className="flex flex-row flex-wrap justify-between items-center gap-y-1 w-full">
            <h3 className="text-2xs font-semibold text-muted-foreground uppercase tracking-wider">Active Inbound Ports</h3>
            <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-[10px] text-foreground font-medium">
              {[
                ...(settings.useSeparatePorts
                  ? [
                      { label: 'HTTP', port: settings.httpPort },
                      { label: 'SOCKS5', port: settings.socksPort },
                    ]
                  : [
                      { label: 'Mixed', port: settings.mixedPort },
                    ]),
                ...(settings.apiEnabled ? [{ label: 'API', port: settings.apiPort }] : []),
              ].map(({ label, port }, i) => (
                <span key={label} className="flex items-center gap-2">
                  {i > 0 && <Separator orientation="vertical" className="h-2.5 hidden sm:block" />}
                  <span className="flex items-center gap-1">
                    <span className="text-muted-foreground font-normal">{label}:</span>
                    <span className="font-mono bg-muted px-1 py-0.5 rounded border border-border/30 text-[10px] tabular-nums">{port}</span>
                  </span>
                </span>
              ))}
              {settings.wifiSharing && (
                <Badge variant="outline" className="h-5 gap-1 text-[9px] text-foreground font-semibold px-1.5 border-border bg-muted/30 leading-none">
                  <Wifi className="size-2.5" />
                  <span>LAN Sharing On</span>
                </Badge>
              )}
            </div>
          </div>
        </div>
      </div>
    </ViewShell>
  );
}
