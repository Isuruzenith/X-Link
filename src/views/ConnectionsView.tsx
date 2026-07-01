import { useEffect, useState, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Network, Search, X, ShieldAlert, ArrowUpDown } from 'lucide-react';
import { ViewShell } from '../components/ViewShell';
import { useConnectionStore } from '../stores/connectionStore';
import { useToastStore } from '../stores/toastStore';

interface ConnectionMetadata {
  network: string;
  type: string;
  sourceIP: string;
  sourcePort: string;
  destinationIP: string;
  destinationPort: string;
  host: string;
}

interface ConnectionEntry {
  id: string;
  metadata: ConnectionMetadata;
  upload: number;
  download: number;
  start: string; // ISO string
  chains: string[];
  rule: string;
  // Computed local speeds
  upSpeed?: number;
  downSpeed?: number;
}

const formatBytes = (bytes: number): string => {
  if (bytes === 0) return '0 B';
  const k = 1024, sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
};

const formatSpeed = (bps: number): string => {
  if (!bps || bps === 0) return '0 B/s';
  const k = 1024, sizes = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
  const i = Math.floor(Math.log(bps) / Math.log(k));
  return parseFloat((bps / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
};

const formatDuration = (startIso: string): string => {
  try {
    const elapsed = Date.now() - new Date(startIso).getTime();
    const seconds = Math.floor(elapsed / 1000);
    if (seconds < 60) return `${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m ${seconds % 60}s`;
    const hours = Math.floor(minutes / 60);
    return `${hours}h ${minutes % 60}m`;
  } catch {
    return '0s';
  }
};

export function ConnectionsView() {
  const { isConnected } = useConnectionStore();
  const [connections, setConnections] = useState<ConnectionEntry[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [filterType, setFilterType] = useState<'all' | 'proxy' | 'direct'>('all');
  const [sortBy, setSortBy] = useState<'host' | 'speed' | 'bytes' | 'time'>('bytes');
  const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('desc');

  const prevBytesRef = useRef<Record<string, { upload: number; download: number; time: number }>>({});

  useEffect(() => {
    if (!isConnected) return;

    const fetchConnections = async () => {
      try {
        const res = await invoke<{ connections: ConnectionEntry[] }>('get_active_connections');
        const list = res?.connections || [];
        const now = Date.now();

        // Calculate per-connection speed
        const updatedList = list.map((conn) => {
          const prev = prevBytesRef.current[conn.id];
          let upSpeed = 0;
          let downSpeed = 0;
          if (prev) {
            const elapsed = (now - prev.time) / 1000;
            if (elapsed > 0) {
              upSpeed = Math.max(0, (conn.upload - prev.upload) / elapsed);
              downSpeed = Math.max(0, (conn.download - prev.download) / elapsed);
            }
          }
          return { ...conn, upSpeed, downSpeed };
        });

        // Store bytes and times in ref for the next interval
        const nextRefs: Record<string, { upload: number; download: number; time: number }> = {};
        list.forEach((conn) => {
          nextRefs[conn.id] = { upload: conn.upload, download: conn.download, time: now };
        });
        prevBytesRef.current = nextRefs;

        setConnections(updatedList);
      } catch {
        // Silent catch to prevent UI crash on boot or network drop
      }
    };

    fetchConnections();
    const interval = setInterval(fetchConnections, 1500);
    return () => clearInterval(interval);
  }, [isConnected]);

  const visibleConnections = isConnected ? connections : [];

  // Kill connection handler
  const handleKill = async (id: string, dest: string) => {
    try {
      await invoke('close_connection', { id });
      setConnections((prev) => prev.filter((c) => c.id !== id));
      useToastStore.getState().addToast('success', `Closed connection to ${dest}`, 'Connection Closed');
    } catch (e) {
      useToastStore.getState().addToast('error', `Failed to close connection: ${e}`, 'Error');
    }
  };

  // Kill all connections handler
  const handleKillAll = async () => {
    if (window.confirm('Are you sure you want to close all active proxy connections?')) {
      try {
        await invoke('close_all_connections');
        setConnections([]);
        useToastStore.getState().addToast('success', 'Closed all active connections', 'Connections Reset');
      } catch (e) {
        useToastStore.getState().addToast('error', `Failed to close all connections: ${e}`, 'Error');
      }
    }
  };

  // Filtering
  const filtered = visibleConnections.filter((c) => {
    // Search query matching
    const query = searchQuery.toLowerCase();
    const matchesSearch =
      (c.metadata.host || '').toLowerCase().includes(query) ||
      (c.metadata.destinationIP || '').toLowerCase().includes(query) ||
      (c.rule || '').toLowerCase().includes(query) ||
      (c.chains.join(' > ') || '').toLowerCase().includes(query);

    if (!matchesSearch) return false;

    // Type matching (direct or proxy)
    const isDirect = c.chains.includes('direct');
    if (filterType === 'proxy' && isDirect) return false;
    if (filterType === 'direct' && !isDirect) return false;

    return true;
  });

  // Sorting
  const sorted = [...filtered].sort((a, b) => {
    let valA: string | number = 0;
    let valB: string | number = 0;

    if (sortBy === 'host') {
      valA = a.metadata.host || a.metadata.destinationIP || '';
      valB = b.metadata.host || b.metadata.destinationIP || '';
    } else if (sortBy === 'speed') {
      valA = (a.upSpeed || 0) + (a.downSpeed || 0);
      valB = (b.upSpeed || 0) + (b.downSpeed || 0);
    } else if (sortBy === 'bytes') {
      valA = a.upload + a.download;
      valB = b.upload + b.download;
    } else if (sortBy === 'time') {
      valA = new Date(a.start).getTime();
      valB = new Date(b.start).getTime();
    }

    if (valA < valB) return sortOrder === 'asc' ? -1 : 1;
    if (valA > valB) return sortOrder === 'asc' ? 1 : -1;
    return 0;
  });

  const toggleSort = (field: typeof sortBy) => {
    if (sortBy === field) {
      setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc');
    } else {
      setSortBy(field);
      setSortOrder('desc');
    }
  };

  return (
    <ViewShell
      title="Connections"
      subtitle="Real-time network tunnels audit and packet statistics"
      actions={
        <div className="flex-row gap-12">
          {visibleConnections.length > 0 && (
            <button className="btn danger sm" onClick={handleKillAll}>
              <ShieldAlert size={13} /> Close All
            </button>
          )}
        </div>
      }
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: '16px', height: 'calc(100vh - 130px)' }}>
        
        {/* Controls row */}
        <div style={{ display: 'flex', gap: '12px', alignItems: 'center', flexShrink: 0 }}>
          <div style={{ position: 'relative', flex: 1, maxWidth: '320px' }}>
            <Search size={14} style={{ position: 'absolute', left: '12px', top: '50%', transform: 'translateY(-50%)', color: 'var(--text-low)' }} />
            <input
              className="text-input"
              style={{ paddingLeft: '32px', fontSize: '12.5px' }}
              placeholder="Filter by host, IP, rule..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
          </div>
          
          <div className="seg-control">
            {([
              { key: 'all', label: 'All Tunnels' },
              { key: 'proxy', label: 'Proxy Tunnels' },
              { key: 'direct', label: 'Direct Tunnels' },
            ] as const).map((f) => (
              <div
                key={f.key}
                className={`seg-item ${filterType === f.key ? 'active' : ''}`}
                onClick={() => setFilterType(f.key)}
              >
                {f.label}
              </div>
            ))}
          </div>

          <span style={{ fontSize: '12px', color: 'var(--text-low)', marginLeft: 'auto', fontWeight: 500 }}>
            {sorted.length} / {visibleConnections.length} active sockets
          </span>
        </div>

        {/* Connections table panel */}
        <div
          className="glass-panel"
          style={{
            flex: 1,
            display: 'flex',
            flexDirection: 'column',
            padding: 0,
            overflow: 'hidden',
            background: 'rgba(10, 11, 14, 0.65)',
            border: '1px solid rgba(255, 255, 255, 0.08)',
            boxShadow: 'inset 0 1px 0 rgba(255, 255, 255, 0.05), var(--shadow-lg)'
          }}
        >
          <div style={{ flex: 1, overflow: 'auto' }}>
            {sorted.length > 0 ? (
              <table className="data-table" style={{ width: '100%' }}>
                <thead>
                  <tr style={{ background: 'rgba(0,0,0,0.1)' }}>
                    <th style={{ padding: '12px 16px', cursor: 'pointer' }} onClick={() => toggleSort('host')}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                        <span>Destination</span>
                        <ArrowUpDown size={10} />
                      </div>
                    </th>
                    <th style={{ padding: '12px 16px' }}>Network / Matched Rule</th>
                    <th style={{ padding: '12px 16px', cursor: 'pointer' }} onClick={() => toggleSort('speed')}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                        <span>Live Speed</span>
                        <ArrowUpDown size={10} />
                      </div>
                    </th>
                    <th style={{ padding: '12px 16px', cursor: 'pointer' }} onClick={() => toggleSort('bytes')}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                        <span>Total Data</span>
                        <ArrowUpDown size={10} />
                      </div>
                    </th>
                    <th style={{ padding: '12px 16px', cursor: 'pointer' }} onClick={() => toggleSort('time')}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                        <span>Uptime</span>
                        <ArrowUpDown size={10} />
                      </div>
                    </th>
                    <th style={{ padding: '12px 16px', width: '50px', textAlign: 'center' }}>Kill</th>
                  </tr>
                </thead>
                <tbody>
                  {sorted.map((conn) => {
                    const host = conn.metadata.host || conn.metadata.destinationIP;
                    const port = conn.metadata.destinationPort;
                    const isDirect = conn.chains.includes('direct');
                    const chainDisplay = conn.chains.join(' \u203a ');
                    
                    return (
                      <tr key={conn.id} style={{ borderBottom: '1px solid rgba(255,255,255,0.03)' }}>
                        {/* Target Address */}
                        <td style={{ padding: '12px 16px', maxWidth: '280px', fontWeight: 300 }}>
                          <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
                            <div style={{ fontWeight: 400, color: 'var(--text-high)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }} title={`${host}:${port}`}>
                              {host}
                              <span style={{ color: 'var(--text-low)', fontWeight: 300 }}>:{port}</span>
                            </div>
                            <div style={{ display: 'flex', gap: '6px', alignItems: 'center' }}>
                              <span style={{
                                fontSize: '9px', fontWeight: 600, padding: '1px 4px', borderRadius: '3px',
                                background: conn.metadata.network === 'udp' ? 'rgba(168, 85, 247, 0.15)' : 'rgba(59, 130, 246, 0.15)',
                                color: conn.metadata.network === 'udp' ? '#a855f7' : '#3b82f6',
                                textTransform: 'uppercase'
                              }}>
                                {conn.metadata.network}
                              </span>
                              <span style={{ fontSize: '10px', color: 'var(--text-low)', fontWeight: 300 }}>
                                type: {conn.metadata.type}
                              </span>
                            </div>
                          </div>
                        </td>

                        {/* Matching Route Rule */}
                        <td style={{ padding: '12px 16px', fontWeight: 300 }}>
                          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                            <div style={{ display: 'flex', alignItems: 'center' }}>
                              <span style={{
                                fontSize: '10px', fontWeight: 500, padding: '2px 6px', borderRadius: '4px', border: '1px solid',
                                background: isDirect ? 'var(--status-ok-dim)' : 'var(--accent-primary-dim)',
                                color: isDirect ? 'var(--status-ok)' : 'var(--accent-primary)',
                                borderColor: isDirect ? 'rgba(34,197,94,0.15)' : 'rgba(74,158,255,0.15)'
                              }}>
                                {chainDisplay}
                              </span>
                            </div>
                            {conn.rule && (
                              <div style={{ fontSize: '10px', color: 'var(--text-low)', fontFamily: 'var(--font-mono)', fontWeight: 300 }} title={conn.rule}>
                                rule: {conn.rule}
                              </div>
                            )}
                          </div>
                        </td>

                        {/* Real-time calculated speeds */}
                        <td style={{ padding: '12px 16px', fontFamily: 'var(--font-mono)', fontWeight: 300 }}>
                          <div style={{ display: 'flex', flexDirection: 'column', gap: '2px', fontSize: '11.5px', fontWeight: 300 }}>
                            <span style={{ color: 'var(--status-ok)' }}>↓ {formatSpeed(conn.downSpeed || 0)}</span>
                            <span style={{ color: 'var(--accent-secondary)' }}>↑ {formatSpeed(conn.upSpeed || 0)}</span>
                          </div>
                        </td>

                        {/* Total bytes stats */}
                        <td style={{ padding: '12px 16px', fontFamily: 'var(--font-mono)', fontWeight: 300 }}>
                          <div style={{ display: 'flex', flexDirection: 'column', gap: '2px', fontSize: '11.5px', color: 'var(--text-med)', fontWeight: 300 }}>
                            <span>↓ {formatBytes(conn.download)}</span>
                            <span>↑ {formatBytes(conn.upload)}</span>
                          </div>
                        </td>

                        {/* Connection lifetime */}
                        <td style={{ padding: '12px 16px', color: 'var(--text-high)', fontSize: '12px', fontWeight: 300 }}>
                          {formatDuration(conn.start)}
                        </td>

                        {/* Termination */}
                        <td style={{ padding: '12px 16px', textAlign: 'center' }}>
                          <button
                            onClick={() => handleKill(conn.id, `${host}:${port}`)}
                            style={{
                              background: 'transparent', border: 'none', color: 'var(--text-low)', cursor: 'pointer',
                              width: '28px', height: '28px', borderRadius: '50%', display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
                              transition: 'all 0.15s'
                            }}
                            onMouseEnter={(e) => {
                              e.currentTarget.style.background = 'rgba(239, 68, 68, 0.15)';
                              e.currentTarget.style.color = '#ef4444';
                            }}
                            onMouseLeave={(e) => {
                              e.currentTarget.style.background = 'transparent';
                              e.currentTarget.style.color = 'var(--text-low)';
                            }}
                            title="Force close connection"
                          >
                            <X size={14} />
                          </button>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            ) : (
              <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--text-low)', padding: '40px' }}>
                <Network size={44} style={{ strokeWidth: 1, marginBottom: '12px', opacity: 0.6 }} />
                <span style={{ fontSize: '14px', fontWeight: 500 }}>
                  {!isConnected ? 'Proxy service is disconnected.' : 'No active network connections found.'}
                </span>
                {isConnected && searchQuery && (
                  <span style={{ fontSize: '12px', marginTop: '4px' }}>Try modifying your filter query.</span>
                )}
              </div>
            )}
          </div>
        </div>
      </div>
    </ViewShell>
  );
}
