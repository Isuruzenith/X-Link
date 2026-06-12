import {
  DownloadCloud, UploadCloud, Clock, Server, Power,
  Shield, ShieldAlert, Network, Wifi, Edit3
} from 'lucide-react';
import { TrafficChart } from '../components/TrafficChart';
import { ViewShell } from '../components/ViewShell';
import type { Profile, Settings } from '../utils/store';

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
  isConnected: boolean;
  connectionStatus: 'connected' | 'connecting' | 'disconnected';
  activeProfileId: string | null;
  uptime: number;
  httpPort: number;
  socksPort: number;
  mixedPort: number;
  isElevated: boolean;
  singboxVersion: string;
  appVersion: string;
  uploadBytes: number;
  downloadBytes: number;
  uploadSpeed: number;
  downloadSpeed: number;
  activeConnections: number;
  speedHistory: { up: number; down: number }[];
  profiles: Profile[];
  settings: Settings;
  selectedProfileId: string | null;
  onToggleConnect: () => void;
  onRequestElevation: () => void;
}

export function DashboardView({
  isConnected, connectionStatus, activeProfileId, uptime, httpPort, socksPort, mixedPort,
  isElevated, uploadBytes, downloadBytes, uploadSpeed,
  downloadSpeed, activeConnections, speedHistory, profiles, settings,
  selectedProfileId,
  onToggleConnect, onRequestElevation,
}: DashboardViewProps) {
  return (
    <ViewShell
      title="Dashboard"
      subtitle="Real-time status monitor and proxy controls"
      actions={
        <div className="flex-row">
          {isElevated ? (
            <div className="tun-shield-badge"><Shield size={14} /><span>TUN: Active</span></div>
          ) : (
            <button className="tun-shield-btn" onClick={onRequestElevation}>
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
              <button className={`power-button ${connectionStatus === 'connected' ? 'connected' : connectionStatus === 'connecting' ? 'connecting' : ''}`} onClick={onToggleConnect}>
                <Power size={28} />
              </button>
            </div>
            <h3 className="connect-status-label">
              {connectionStatus === 'connected' ? 'Connected' : connectionStatus === 'connecting' ? 'Connecting...' : 'Disconnected'}
            </h3>
            <p className="connect-status-sub">
              {connectionStatus === 'connected'
                ? `${profiles.find((p) => p.id === activeProfileId)?.name || 'Default'}`
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
            <div key={label} className="glass-panel metric-card">
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
                { label: 'HTTP',   port: httpPort },
                { label: 'SOCKS5', port: socksPort },
                { label: 'Mixed',  port: mixedPort },
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
