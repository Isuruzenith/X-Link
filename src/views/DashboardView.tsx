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

  return (
    <ViewShell
      title="Dashboard"
      subtitle="Real-time status monitor and proxy controls"
      actions={
        <div className="flex-row">
          {isElevated ? (
            <div className="tun-shield-badge"><Shield size={14} /><span>TUN: Active</span></div>
          ) : (
            <button className="tun-shield-btn" onClick={requestElevation}>
              <ShieldAlert size={14} /><span>TUN: Elevation Required</span>
            </button>
          )}
        </div>
      }
    >
      <div className="view-container">
        {/* Top row: Connect + Chart */}
        <div className="grid-3">
          {/* Connect Panel */}
          <div className="glass-panel connect-panel">
            <div className="power-button-container">
              <div className={`power-button-outer ${connectionStatus === 'connected' ? 'connected' : connectionStatus === 'connecting' ? 'connecting' : ''}`} />
              <button
                className={`power-button ${connectionStatus === 'connected' ? 'connected' : connectionStatus === 'connecting' ? 'connecting' : ''}`}
                onClick={toggleConnect}
              >
                <Power size={28} />
              </button>
            </div>
            <h3 className="connect-status-label">
              {connectionStatus === 'connected' ? 'Connected' : connectionStatus === 'connecting' ? 'Connecting...' : 'Disconnected'}
            </h3>
            <p className="connect-status-sub">
              {connectionStatus === 'connected'
                ? `${activeProfile?.name || 'Default'}`
                : connectionStatus === 'connecting'
                  ? 'Establishing secure tunnels...'
                  : 'Toggle power to start proxy'}
            </p>
            {isConnected && (
              <div className="connect-mode-badge">
                <Network size={12} />
                <span>{settings.proxyMode === 'tun' ? 'TUN Mode' : 'System Proxy'}</span>
              </div>
            )}
            {isConnected && selectedNodeTag && (
              <div style={{ marginTop: '8px', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '6px' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '4px', fontSize: '11px', color: 'var(--text-low)' }}>
                  <Zap size={10} style={{ color: 'var(--accent-primary)' }} />
                  <span style={{ color: 'var(--text-med)', fontWeight: 600 }}>{selectedNodeTag}</span>
                </div>
                {(() => {
                  const geo = activeNode ? nodeGeoCache[activeNode.server] : null;
                  const activeServerCode = geo && geo !== 'loading'
                    ? geo.countryCode
                    : (activeNode ? getCountryCode(activeNode.tag) : null);
                  const activeRegion = geo && geo !== 'loading' && (geo.cityName || geo.regionName)
                    ? `${geo.cityName || ''}${geo.cityName && geo.regionName ? ', ' : ''}${geo.regionName || ''}`
                    : '';
                  const activeCountryName = geo && geo !== 'loading' ? geo.countryName : '';

                  if (!activeServerCode) return null;
                  return (
                    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '4px', marginTop: '2px' }}>
                      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '16px' }}>
                        <img
                          src={`https://flagcdn.com/w40/${activeServerCode.toLowerCase()}.png`}
                          alt={activeServerCode}
                          style={{ width: '24px', height: '16px', objectFit: 'cover', borderRadius: '2px', boxShadow: '0 1px 3px rgba(0,0,0,0.25)' }}
                          onError={(e) => {
                            (e.target as HTMLImageElement).style.display = 'none';
                          }}
                        />
                      </div>
                      {(activeCountryName || activeRegion) && (
                        <span style={{ fontSize: '10px', color: 'var(--text-low)', textAlign: 'center', maxWidth: '180px', wordBreak: 'break-all' }}>
                          {activeCountryName}{activeCountryName && activeRegion ? ' - ' : ''}{activeRegion}
                        </span>
                      )}
                    </div>
                  );
                })()}
              </div>
            )}
            {isConnected && nodes.length > 1 && (
              <div style={{
                marginTop: '12px', display: 'flex', gap: '4px', flexWrap: 'wrap', justifyContent: 'center',
                maxHeight: '52px', overflowY: 'auto', overflowX: 'hidden',
              }}>
                {nodes.slice(0, 8).map((node) => (
                  <button
                    key={node.tag}
                    onClick={() => selectNode(node)}
                    style={{
                      fontSize: '9px', padding: '2px 8px', borderRadius: '10px', border: 'none', cursor: 'pointer',
                      background: selectedNodeTag === node.tag ? 'var(--accent-primary)' : 'var(--surface-sunken)',
                      color: selectedNodeTag === node.tag ? '#fff' : 'var(--text-low)',
                      transition: 'all 0.15s',
                      whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', maxWidth: '100px',
                    }}
                    title={node.tag}
                  >
                    {node.tag}
                  </button>
                ))}
                {nodes.length > 8 && (
                  <span style={{ fontSize: '9px', color: 'var(--text-low)', padding: '2px 4px' }}>+{nodes.length - 8} more</span>
                )}
              </div>
            )}
          </div>

          {/* Chart */}
          <div className="glass-panel chart-panel" style={{ gridColumn: 'span 2' }}>
            <div className="chart-header">
              <h3 className="chart-title">Bandwidth</h3>
              <div className="chart-legends">
                <div className="legend-item"><div className="legend-dot download" /><span>↓ {formatSpeed(downloadSpeed)}</span></div>
                <div className="legend-item"><div className="legend-dot upload" /><span>↑ {formatSpeed(uploadSpeed)}</span></div>
              </div>
            </div>
            <div className="chart-body"><TrafficChart history={speedHistory} /></div>
          </div>
        </div>

        {/* Metrics */}
        <div className="grid-4">
          {[
            { icon: DownloadCloud, label: 'Total Down',   value: formatBytes(downloadBytes),    color: 'var(--accent-cyan)' },
            { icon: UploadCloud,   label: 'Total Up',     value: formatBytes(uploadBytes),       color: 'var(--accent-purple)' },
            { icon: Clock,         label: 'Uptime',       value: formatUptime(uptime),            color: 'var(--status-warn)' },
            { icon: Server,        label: 'Connections',  value: `${activeConnections} active`,  color: 'var(--status-ok)' },
          ].map(({ icon: Icon, label, value, color }) => (
            <div
              key={label}
              className={`glass-panel metric-card ${label === 'Connections' ? 'clickable' : ''}`}
              onClick={label === 'Connections' ? () => onNavigateToTab?.('connections') : undefined}
            >
              <div className="metric-icon-box" style={{ color }}><Icon size={20} /></div>
              <div className="metric-info">
                <span className="metric-label">{label}</span>
                <span className="metric-value">{value}</span>
              </div>
            </div>
          ))}
        </div>

        {/* Ports bar */}
        <div className="glass-panel" style={{ padding: '14px 24px' }}>
          <div className="flex-row-between">
            <h3 style={{ fontSize: '13px', fontWeight: 600, color: 'var(--text-med)' }}>Active Inbound Ports</h3>
            <div style={{ display: 'flex', gap: '24px', fontSize: '13px' }}>
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
              ].map(({ label, port }) => (
                <span key={label}>
                  <span style={{ color: 'var(--text-low)' }}>{label}: </span>
                  <strong style={{ color: 'var(--accent-primary)', fontFamily: 'var(--font-mono)' }}>{port}</strong>
                </span>
              ))}
              {settings.wifiSharing && (
                <span style={{ color: 'var(--status-ok)', display: 'flex', alignItems: 'center', gap: '4px', fontSize: '12px' }}>
                  <Wifi size={12} /> LAN Sharing On
                </span>
              )}
            </div>
          </div>
        </div>
      </div>
    </ViewShell>
  );
}
