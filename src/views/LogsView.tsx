import React, { useRef, useEffect, useState } from 'react';
import { Terminal, Copy, Trash2 } from 'lucide-react';
import { ViewShell } from '../components/ViewShell';

interface LogEntry { type: 'info' | 'warn' | 'error' | 'system'; text: string; }

interface LogsViewProps {
  logs: LogEntry[];
  autoScroll: boolean;
  onSetAutoScroll: (v: boolean) => void;
  onClearLogs: () => void;
  onCopyLogs: () => void;
}

export function LogsView({ logs, autoScroll, onSetAutoScroll, onClearLogs, onCopyLogs }: LogsViewProps) {
  const endRef = useRef<HTMLDivElement>(null);
  const [filterText, setFilterText] = useState('');
  const [levelFilter, setLevelFilter] = useState<'all' | 'info' | 'warn' | 'error'>('all');

  useEffect(() => {
    if (autoScroll && endRef.current) endRef.current.scrollIntoView({ behavior: 'smooth' });
  }, [logs, autoScroll]);

  const filtered = logs.filter((l) => {
    if (levelFilter !== 'all' && l.type !== levelFilter) return false;
    if (filterText && !l.text.toLowerCase().includes(filterText.toLowerCase())) return false;
    return true;
  });

  const getLevel = (type: string) => {
    if (type === 'error') return 'ERR';
    if (type === 'warn')  return 'WARN';
    if (type === 'system') return 'SYS';
    return 'INFO';
  };

  return (
    <ViewShell
      title="Logs"
      subtitle="Real-time sing-box process console output"
      actions={
        <div className="flex-row">
          <button className="btn secondary sm" onClick={onCopyLogs}><Copy size={13} /> Copy All</button>
          <button className="btn secondary sm" onClick={onClearLogs}><Trash2 size={13} /> Clear</button>
          <label className="switch-toggle" title="Autoscroll">
            <input type="checkbox" checked={autoScroll} onChange={(e) => onSetAutoScroll(e.target.checked)} />
            <span className="switch-slider"></span>
          </label>
          <span style={{ fontSize: '11px', color: 'var(--text-low)' }}>Autoscroll</span>
        </div>
      }
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: '12px', height: 'calc(100vh - 120px)' }}>
        {/* Filter bar */}
        <div style={{ display: 'flex', gap: '12px', alignItems: 'center', flexShrink: 0 }}>
          <input
            className="text-input"
            style={{ maxWidth: '300px', fontFamily: 'var(--font-mono)', fontSize: '12px' }}
            placeholder="Filter logs…"
            value={filterText}
            onChange={(e) => setFilterText(e.target.value)}
          />
          <div className="seg-control">
            {(['all', 'info', 'warn', 'error'] as const).map((l) => (
              <div key={l} className={`seg-item ${levelFilter === l ? 'active' : ''}`} onClick={() => setLevelFilter(l)}
                style={l === 'error' && levelFilter === l ? { color: 'var(--status-err)' } : l === 'warn' && levelFilter === l ? { color: 'var(--status-warn)' } : undefined}>
                {l.toUpperCase()}
              </div>
            ))}
          </div>
          <span style={{ fontSize: '11px', color: 'var(--text-low)', marginLeft: 'auto' }}>
            {filtered.length} / {logs.length} lines
          </span>
        </div>

        {/* Terminal */}
        <div className="glass-panel" style={{ flex: 1, display: 'flex', flexDirection: 'column', padding: 0, overflow: 'hidden' }}>
          <div style={{ flex: 1, overflow: 'auto', padding: '12px 16px', fontFamily: 'var(--font-mono)', fontSize: '12px', lineHeight: 1.7 }}>
            {filtered.length > 0 ? filtered.map((log, i) => (
              <div key={i} className={`log-line ${log.type}`}>
                <span className="log-level" style={{
                  color: log.type === 'error' ? 'var(--status-err)' :
                         log.type === 'warn'  ? 'var(--status-warn)' :
                         log.type === 'system' ? 'var(--text-low)' : 'var(--status-info)'
                }}>
                  {getLevel(log.type)}
                </span>
                <span className="log-text">{log.text}</span>
              </div>
            )) : (
              <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--text-low)' }}>
                <Terminal size={32} style={{ strokeWidth: 1, marginBottom: '8px' }} />
                <span style={{ fontSize: '13px' }}>
                  {logs.length > 0 ? 'No logs match the current filter.' : 'Waiting for process logs…'}
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
