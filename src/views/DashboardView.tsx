import { useEffect } from 'react';
import { Icon } from '@/components/ui/icon';
import { TrafficChart } from '../components/TrafficChart';
import { ViewShell } from '../components/ViewShell';
import { useConnectionStore } from '../stores/connectionStore';
import { useSettingsStore } from '../stores/settingsStore';
import { useProfileStore } from '../stores/profileStore';
import { Button } from '@/components/ui/button';

const formatBytes = (bytes: number): string => {
  if (!bytes) return '0 B';
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), 4);
  return `${(bytes / 1024 ** i).toFixed(1)} ${['B', 'KB', 'MB', 'GB', 'TB'][i]}`;
};

const formatSpeed = (bytes: number) => `${formatBytes(bytes)}/s`;

const formatUptime = (seconds: number): string => {
  const h = Math.floor(seconds / 3600).toString().padStart(2, '0');
  const m = Math.floor((seconds % 3600) / 60).toString().padStart(2, '0');
  const s = (seconds % 60).toString().padStart(2, '0');
  return `${h}:${m}:${s}`;
};

interface DashboardViewProps {
  onNavigateToTab?: (tab: 'profiles' | 'connections' | 'settings') => void;
}

export function DashboardView({ onNavigateToTab }: DashboardViewProps) {
  const {
    connectionStatus, effectiveProxyMode, uptime, uploadBytes, downloadBytes,
    uploadSpeed, downloadSpeed, activeConnections, speedHistory, corePid, memoryBytes, cpuPercent,
    toggleConnect, requestElevation,
  } = useConnectionStore();
  const { settings, isElevated } = useSettingsStore();
  const activeProfile = useProfileStore((s) => s.activeProfile)();
  const { nodes, selectedNodeTag, selectNode, selectProfile } = useProfileStore();
  const activeProfileId = activeProfile?.id;
  const activeNode = nodes.find((node) => node.tag === selectedNodeTag);
  const connected = connectionStatus === 'connected';
  const busy = connectionStatus === 'connecting';
  const statusLabel = connected ? 'Connected' : busy ? 'Connecting' : 'Disconnected';

  useEffect(() => {
    if (activeProfileId) selectProfile(activeProfileId);
  }, [activeProfileId, selectProfile]);

  return (
    <ViewShell title="Dashboard" subtitle="Your connection at a glance" actions={
      <span className={`dashboard-status ${connectionStatus}`} role="status">
        <span aria-hidden="true" className="session-indicator" />{statusLabel}
      </span>
    }>
      <div className="dashboard-layout">
        <section className={`dashboard-connect ${connectionStatus}`} aria-label="Connection control">
          <div className="dashboard-connect-header">
            <span className="dashboard-connect-symbol"><Icon name="power_settings_new" size={26} aria-hidden="true" /></span>
            <span className="dashboard-connect-mode">{connected
              ? effectiveProxyMode === 'tun' ? 'TUN routing' : effectiveProxyMode === 'system' ? 'System proxy' : 'Mode unknown'
              : settings.proxyMode === 'tun' ? 'TUN preferred' : 'System proxy preferred'}</span>
          </div>
          <div className="dashboard-connect-body">
            <h2>{statusLabel}</h2>
            <p>{busy ? 'Starting the proxy and checking your connection…' : connected
              ? `Traffic is routed through ${activeNode?.tag || selectedNodeTag || activeProfile?.name || 'your selected server'}.`
              : activeProfile ? 'Connect to route traffic through your selected server.' : 'Add a profile to start connecting.'}</p>
            {connected && settings.proxyMode === 'tun' && effectiveProxyMode === 'system' && (
              <p className="mode-fallback" role="status">TUN was unavailable. This session is using System Proxy instead.</p>
            )}
          </div>
          <div className="dashboard-connect-actions">
            <Button onClick={toggleConnect} disabled={busy || (!connected && !activeProfile)} aria-label={connected ? 'Disconnect proxy' : 'Connect proxy'} className="dashboard-primary-action">
              {busy && <span className="button-spinner" aria-hidden="true" />}
              <span>{busy ? 'Connecting…' : connected ? 'Disconnect' : 'Connect'}</span>
            </Button>
            {!activeProfile && !connected && <Button variant="outline" onClick={() => onNavigateToTab?.('profiles')}>Add a profile</Button>}
          </div>
          <div className="dashboard-connect-footer">
            {settings.proxyMode === 'tun' && !isElevated && !connected ? (
              <Button variant="ghost" onClick={requestElevation} className="dashboard-admin-action">
                <Icon name="warning" size={17} aria-hidden="true" /> Run as administrator for TUN
              </Button>
            ) : (
              <span><Icon name="verified_user" size={16} aria-hidden="true" />{connected ? `Session active · ${formatUptime(uptime)}` : isElevated ? 'Administrator access available' : 'Ready to connect'}</span>
            )}
          </div>
        </section>

        <section className="dashboard-traffic" aria-labelledby="traffic-title">
          <div className="dashboard-section-heading">
            <div><h2 id="traffic-title">Live traffic</h2><p>Last 30 seconds · bytes per second</p></div>
            <span className="live-tag">{connected ? 'Live' : 'Idle'}</span>
          </div>
          <div className="dashboard-speeds" aria-label="Current transfer speeds">
            <div><span className="speed-key down"><Icon name="arrow_downward" size={16} aria-hidden="true" />Download</span><strong>{formatSpeed(downloadSpeed)}</strong></div>
            <div><span className="speed-key up"><Icon name="arrow_upward" size={16} aria-hidden="true" />Upload</span><strong>{formatSpeed(uploadSpeed)}</strong></div>
          </div>
          <div className="dashboard-plot"><TrafficChart history={speedHistory} /></div>
          <details className="traffic-data"><summary>View traffic data as a table</summary>
            <div className="traffic-data-scroll"><table><caption>Recent traffic speeds</caption><thead><tr><th scope="col">Seconds ago</th><th scope="col">Download</th><th scope="col">Upload</th></tr></thead><tbody>
              {speedHistory.slice(-30).map((point, index, samples) => <tr key={index}><th scope="row">{samples.length - index - 1}</th><td>{formatSpeed(point.down)}</td><td>{formatSpeed(point.up)}</td></tr>)}
            </tbody></table></div>
          </details>
        </section>

        <section className="dashboard-server" aria-labelledby="server-title">
          <div className="dashboard-section-heading"><div><h2 id="server-title">Active server</h2><p>{activeProfile?.name || 'No profile selected'}</p></div><Icon name="dns" size={20} aria-hidden="true" /></div>
          {nodes.length > 0 ? <>
            <div className="dashboard-server-name"><span className="server-glyph"><Icon name="router" size={21} aria-hidden="true" /></span><div><strong title={activeNode?.tag || selectedNodeTag || nodes[0].tag}>{activeNode?.tag || selectedNodeTag || nodes[0].tag}</strong><span>{activeNode ? `${activeNode.server}:${activeNode.port}` : 'Select a server below'}</span></div></div>
            {nodes.length > 1 && <div className="dashboard-server-list" aria-label="Switch server">
              {nodes.slice(0, 8).map((node) => <button key={node.tag} type="button" className={`server-choice ${selectedNodeTag === node.tag ? 'is-selected' : ''}`} aria-pressed={selectedNodeTag === node.tag} onClick={() => selectNode(node)} title={node.tag}>{node.tag}</button>)}
              {nodes.length > 8 && <Button variant="ghost" onClick={() => onNavigateToTab?.('profiles')}>View all {nodes.length} servers</Button>}
            </div>}
          </> : <p className="dashboard-empty">Import a profile to see available servers.</p>}
        </section>

        <section className="dashboard-details" aria-label="Session details">
          <div className="dashboard-detail"><Icon name="arrow_downward" aria-hidden="true" size={20} /><span>Downloaded</span><strong>{formatBytes(downloadBytes)}</strong></div>
          <div className="dashboard-detail"><Icon name="arrow_upward" aria-hidden="true" size={20} /><span>Uploaded</span><strong>{formatBytes(uploadBytes)}</strong></div>
          <div className="dashboard-detail"><Icon name="schedule" aria-hidden="true" size={20} /><span>Uptime</span><strong>{connected ? formatUptime(uptime) : '—'}</strong></div>
          <button type="button" className="dashboard-detail detail-link" onClick={() => onNavigateToTab?.('connections')}><Icon name="router" aria-hidden="true" size={20} /><span>Connections</span><strong>{activeConnections} active</strong></button>
          <div className="dashboard-detail" title={connected && corePid ? `sing-box PID: ${corePid}` : undefined}>
            <Icon name="memory" aria-hidden="true" size={20} />
            <span>Core</span>
            <strong>{connected ? `${formatBytes(memoryBytes)} · ${cpuPercent.toFixed(1)}%` : 'Inactive'}</strong>
          </div>
        </section>

        <section className="dashboard-inbounds" aria-label="Inbound listeners">
          <strong>Inbound listeners</strong>
          <div>{(settings.useSeparatePorts ? [
            { name: 'HTTP', port: settings.httpPort }, { name: 'SOCKS5', port: settings.socksPort },
          ] : [{ name: 'Mixed', port: settings.mixedPort }]).concat(settings.apiEnabled ? [{ name: 'API', port: settings.apiPort }] : []).map(({ name, port }) =>
            <span key={name} className="inbound-port">{name} <code>{port}</code></span>)}
            {settings.wifiSharing && <span className="inbound-port"><Icon name="wifi" size={14} aria-hidden="true" /> LAN sharing on</span>}
          </div>
          <Button variant="ghost" onClick={() => onNavigateToTab?.('settings')}>Settings</Button>
        </section>
      </div>
    </ViewShell>
  );
}
