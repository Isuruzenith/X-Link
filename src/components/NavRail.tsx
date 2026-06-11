import React from 'react';
import { Activity, Layers, Route, Dna, Terminal, Settings, Zap } from 'lucide-react';

export type TabId = 'dashboard' | 'profiles' | 'routing' | 'dns' | 'logs' | 'settings';

interface NavRailProps {
  activeTab: TabId;
  onTabChange: (tab: TabId) => void;
  isConnected: boolean;
  errorLogCount: number;
  singboxVersion: string;
}

const NAV_ITEMS: [TabId, React.ComponentType<any>, string][] = [
  ['dashboard', Activity,  'Dashboard'],
  ['profiles',  Layers,    'Profiles'],
  ['routing',   Route,     'Routing'],
  ['dns',       Dna,       'DNS'],
  ['logs',      Terminal,  'Logs'],
  ['settings',  Settings,  'Settings'],
];

export function NavRail({ activeTab, onTabChange, isConnected, errorLogCount, singboxVersion }: NavRailProps) {
  const versionShort = singboxVersion?.match(/v[\d.]+/)?.[0] ?? '—';

  return (
    <aside className="nav-rail">
      {/* Logo */}
      <div className="rail-logo">
        <div className="rail-logo-icon">
          <Zap size={14} color="#0b0c10" strokeWidth={3} />
        </div>
        <span className="rail-logo-wordmark">TunX</span>
      </div>

      {/* Navigation */}
      <nav className="nav-group">
        {NAV_ITEMS.map(([tab, Icon, label]) => (
          <div
            key={tab}
            className={`nav-item ${activeTab === tab ? 'active' : ''}`}
            onClick={() => onTabChange(tab)}
            title={label}
          >
            <Icon className="nav-icon" size={18} />
            <span className="nav-label">{label}</span>
            {tab === 'logs' && errorLogCount > 0 && (
              <span className="nav-badge err">{errorLogCount > 99 ? '99+' : errorLogCount}</span>
            )}
          </div>
        ))}
      </nav>

      {/* Footer */}
      <div className="rail-footer">
        <div className="rail-status-row">
          <div className={`status-dot ${isConnected ? 'ok' : 'idle'}`} />
          <span
            className="rail-status-text"
            style={{ color: isConnected ? 'var(--status-ok)' : 'var(--status-warn)' }}
          >
            {isConnected ? 'connected' : 'idle'}
          </span>
        </div>
        <span className="rail-version">{versionShort}</span>
      </div>
    </aside>
  );
}
