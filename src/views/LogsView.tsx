import { useRef, useEffect, useState } from 'react';
import { Terminal, Copy, Trash2, Search, Info, AlertTriangle, ShieldAlert, Cpu } from 'lucide-react';
import { ViewShell } from '../components/ViewShell';
import { useLogStore } from '../stores/logStore';
import { useToastStore } from '../stores/toastStore';

export function LogsView() {
  const { logs, autoScroll, setAutoScroll, clearLogs, copyLogs } = useLogStore();
  const endRef = useRef<HTMLDivElement>(null);
  const [filterText, setFilterText] = useState('');
  const [levelFilter, setLevelFilter] = useState<'all' | 'info' | 'warn' | 'error' | 'system'>('all');

  useEffect(() => {
    if (autoScroll && endRef.current) {
      endRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [logs, autoScroll]);

  // Statistics counters
  const totalCount = logs.length;
  const infoCount = logs.filter((l) => l.type === 'info').length;
  const warnCount = logs.filter((l) => l.type === 'warn').length;
  const errorCount = logs.filter((l) => l.type === 'error').length;
  const systemCount = logs.filter((l) => l.type === 'system').length;

  const filtered = logs.filter((l) => {
    if (levelFilter !== 'all' && l.type !== levelFilter) return false;
    if (filterText && !l.text.toLowerCase().includes(filterText.toLowerCase())) return false;
    return true;
  });

  const handleCopy = async () => {
    await copyLogs();
    useToastStore.getState().addToast('success', 'All logs copied to clipboard.', 'Logs Copied');
  };

  // Syntax highlighting for log tags like [DNS], [TUN], etc.
  const renderHighlightedText = (text: string) => {
    const regex = /(\[[^\]]+\])/g;
    const parts = text.split(regex);
    return parts.map((part, idx) => {
      if (part.startsWith('[') && part.endsWith(']')) {
        const tagText = part.slice(1, -1);
        const lowerTag = tagText.toLowerCase();
        
        let color = '#4a9eff';
        let bg = 'rgba(74, 158, 255, 0.08)';
        let border = 'rgba(74, 158, 255, 0.15)';
        
        if (lowerTag.includes('dns')) {
          color = '#38bdf8'; // Sky blue
          bg = 'rgba(56, 189, 248, 0.08)';
          border = 'rgba(56, 189, 248, 0.15)';
        } else if (lowerTag.includes('tun') || lowerTag.includes('route')) {
          color = '#a855f7'; // Purple
          bg = 'rgba(168, 85, 247, 0.08)';
          border = 'rgba(168, 85, 247, 0.15)';
        } else if (lowerTag.includes('system') || lowerTag.includes('core')) {
          color = '#10b981'; // Green
          bg = 'rgba(16, 185, 129, 0.08)';
          border = 'rgba(16, 185, 129, 0.15)';
        } else if (lowerTag.includes('warn')) {
          color = '#f59e0b'; // Yellow
          bg = 'rgba(245, 158, 11, 0.08)';
          border = 'rgba(245, 158, 11, 0.15)';
        } else if (lowerTag.includes('err') || lowerTag.includes('fail')) {
          color = '#ef4444'; // Red
          bg = 'rgba(239, 68, 68, 0.08)';
          border = 'rgba(239, 68, 68, 0.15)';
        }
        
        return (
          <span
            key={idx}
            style={{
              color,
              background: bg,
              border: `1px solid ${border}`,
              padding: '1px 5px',
              borderRadius: '4px',
              fontSize: '10px',
              fontWeight: 600,
              marginRight: '6px',
              display: 'inline-block',
              textTransform: 'uppercase',
              letterSpacing: '0.5px',
              lineHeight: '1.2'
            }}
          >
            {tagText}
          </span>
        );
      }
      return <span key={idx}>{part}</span>;
    });
  };

  const getLevelLabel = (type: string) => {
    if (type === 'error') return 'ERR';
    if (type === 'warn')  return 'WARN';
    if (type === 'system') return 'SYS';
    return 'INFO';
  };

  return (
    <ViewShell
      title="Console Logs"
      subtitle="Real-time core proxy logs and system events output"
      actions={
        <div className="flex-row gap-12">
          <button className="btn secondary sm" onClick={handleCopy}>
            <Copy size={13} /> Copy All
          </button>
          <button className="btn secondary sm" onClick={clearLogs}>
            <Trash2 size={13} /> Clear
          </button>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px', borderLeft: '1px solid var(--border-subtle)', paddingLeft: '12px' }}>
            <label className="switch-toggle" title="Autoscroll">
              <input type="checkbox" checked={autoScroll} onChange={(e) => setAutoScroll(e.target.checked)} />
              <span className="switch-slider"></span>
            </label>
            <span style={{ fontSize: '11px', color: 'var(--text-low)', fontWeight: 500 }}>Autoscroll</span>
          </div>
        </div>
      }
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: '16px', height: 'calc(100vh - 130px)' }}>
        
        {/* Metric summary tiles */}
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(5, 1fr)', gap: '12px', flexShrink: 0 }}>
          <div className="metric-tile" style={{ display: 'flex', alignItems: 'center', gap: '12px', padding: '10px 14px' }}>
            <div style={{ background: 'rgba(255,255,255,0.05)', color: 'var(--text-med)', padding: '6px', borderRadius: '8px', display: 'flex' }}>
              <Terminal size={16} />
            </div>
            <div>
              <div className="metric-tile-val" style={{ fontSize: '16px', lineHeight: '1.2' }}>{totalCount}</div>
              <div className="metric-tile-label" style={{ fontSize: '9px', marginTop: '1px' }}>Total Logs</div>
            </div>
          </div>
          
          <div className="metric-tile" style={{ display: 'flex', alignItems: 'center', gap: '12px', padding: '10px 14px' }}>
            <div style={{ background: 'rgba(59, 130, 246, 0.08)', color: 'var(--status-info)', padding: '6px', borderRadius: '8px', display: 'flex' }}>
              <Info size={16} />
            </div>
            <div>
              <div className="metric-tile-val" style={{ fontSize: '16px', lineHeight: '1.2', color: 'var(--text-high)' }}>{infoCount}</div>
              <div className="metric-tile-label" style={{ fontSize: '9px', marginTop: '1px' }}>Info Logs</div>
            </div>
          </div>

          <div className="metric-tile" style={{ display: 'flex', alignItems: 'center', gap: '12px', padding: '10px 14px' }}>
            <div style={{ background: 'rgba(245, 158, 11, 0.08)', color: '#f59e0b', padding: '6px', borderRadius: '8px', display: 'flex', boxShadow: '0 0 8px rgba(245, 158, 11, 0.15)' }}>
              <AlertTriangle size={16} />
            </div>
            <div>
              <div className="metric-tile-val" style={{ fontSize: '16px', lineHeight: '1.2', color: '#f59e0b' }}>{warnCount}</div>
              <div className="metric-tile-label" style={{ fontSize: '9px', marginTop: '1px' }}>Warnings</div>
            </div>
          </div>

          <div className="metric-tile" style={{ display: 'flex', alignItems: 'center', gap: '12px', padding: '10px 14px' }}>
            <div style={{ background: 'rgba(239, 68, 68, 0.08)', color: '#ef4444', padding: '6px', borderRadius: '8px', display: 'flex', boxShadow: '0 0 8px rgba(239, 68, 68, 0.15)' }}>
              <ShieldAlert size={16} />
            </div>
            <div>
              <div className="metric-tile-val" style={{ fontSize: '16px', lineHeight: '1.2', color: '#ef4444' }}>{errorCount}</div>
              <div className="metric-tile-label" style={{ fontSize: '9px', marginTop: '1px' }}>Errors</div>
            </div>
          </div>

          <div className="metric-tile" style={{ display: 'flex', alignItems: 'center', gap: '12px', padding: '10px 14px' }}>
            <div style={{ background: 'rgba(16, 185, 129, 0.08)', color: '#10b981', padding: '6px', borderRadius: '8px', display: 'flex' }}>
              <Cpu size={16} />
            </div>
            <div>
              <div className="metric-tile-val" style={{ fontSize: '16px', lineHeight: '1.2', color: '#10b981' }}>{systemCount}</div>
              <div className="metric-tile-label" style={{ fontSize: '9px', marginTop: '1px' }}>System</div>
            </div>
          </div>
        </div>

        {/* Filter bar */}
        <div style={{ display: 'flex', gap: '12px', alignItems: 'center', flexShrink: 0 }}>
          <div style={{ position: 'relative', flex: 1, maxWidth: '320px' }}>
            <Search size={14} style={{ position: 'absolute', left: '12px', top: '50%', transform: 'translateY(-50%)', color: 'var(--text-low)' }} />
            <input
              className="text-input"
              style={{ paddingLeft: '32px', fontSize: '12.5px' }}
              placeholder="Search console output…"
              value={filterText}
              onChange={(e) => setFilterText(e.target.value)}
            />
          </div>
          
          <div className="seg-control">
            {([
              { key: 'all', label: 'All' },
              { key: 'info', label: 'Info' },
              { key: 'warn', label: 'Warn' },
              { key: 'error', label: 'Errors' },
              { key: 'system', label: 'System' }
            ] as const).map((l) => (
              <div
                key={l.key}
                className={`seg-item ${levelFilter === l.key ? 'active' : ''}`}
                onClick={() => setLevelFilter(l.key)}
                style={
                  l.key === 'error' && levelFilter === l.key ? { color: '#ef4444' } :
                  l.key === 'warn' && levelFilter === l.key ? { color: '#f59e0b' } :
                  l.key === 'system' && levelFilter === l.key ? { color: '#10b981' } : undefined
                }
              >
                {l.label}
              </div>
            ))}
          </div>
          
          <span style={{ fontSize: '12px', color: 'var(--text-low)', marginLeft: 'auto', fontWeight: 500 }}>
            Showing {filtered.length} of {logs.length} entries
          </span>
        </div>

        {/* Terminal glass block */}
        <div
          className="glass-panel"
          style={{
            flex: 1,
            display: 'flex',
            flexDirection: 'column',
            padding: 0,
            overflow: 'hidden',
            background: 'rgba(10, 11, 14, 0.75)',
            border: '1px solid rgba(255, 255, 255, 0.08)',
            boxShadow: 'inset 0 1px 0 rgba(255, 255, 255, 0.05), var(--shadow-lg)'
          }}
        >
          <div
            style={{
              flex: 1,
              overflow: 'auto',
              padding: '16px',
              fontFamily: 'var(--font-mono)',
              fontSize: '12px',
              lineHeight: '1.8'
            }}
          >
            {filtered.length > 0 ? (
              filtered.map((log, i) => (
                <div
                  key={i}
                  className={`log-line ${log.type}`}
                  style={{
                    display: 'flex',
                    alignItems: 'flex-start',
                    padding: '4px 6px',
                    borderRadius: '4px',
                    marginBottom: '2px',
                    transition: 'background 0.15s ease'
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = 'rgba(255, 255, 255, 0.03)';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = 'transparent';
                  }}
                >
                  {/* Timestamp */}
                  <span
                    style={{
                      color: 'var(--text-low)',
                      marginRight: '12px',
                      userSelect: 'none',
                      fontSize: '11px',
                      opacity: 0.65,
                      flexShrink: 0
                    }}
                  >
                    {log.timestamp}
                  </span>

                  {/* Level tag */}
                  <span
                    className="log-level"
                    style={{
                      width: '38px',
                      flexShrink: 0,
                      fontWeight: 600,
                      fontSize: '9.5px',
                      letterSpacing: '0.4px',
                      marginRight: '8px',
                      userSelect: 'none',
                      color: log.type === 'error' ? '#ef4444' :
                             log.type === 'warn'  ? '#f59e0b' :
                             log.type === 'system' ? '#10b981' : '#3b82f6'
                    }}
                  >
                    {getLevelLabel(log.type)}
                  </span>

                  {/* Log content */}
                  <span className="log-text" style={{ color: log.type === 'error' ? 'rgba(239, 68, 68, 0.95)' : 'var(--text-med)', flex: 1, wordBreak: 'break-all' }}>
                    {renderHighlightedText(log.text)}
                  </span>
                </div>
              ))
            ) : (
              <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--text-low)' }}>
                <Terminal size={36} style={{ strokeWidth: 1.2, marginBottom: '10px', opacity: 0.7 }} />
                <span style={{ fontSize: '13px', fontWeight: 500 }}>
                  {logs.length > 0 ? 'No logs match the current filters.' : 'Waiting for core process console output…'}
                </span>
              </div>
            )}
            <div ref={endRef} />
          </div>
        </div>
      </div>
    </ViewShell>
  );
}
