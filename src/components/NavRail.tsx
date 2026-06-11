import React from 'react';
import { Activity, Layers, Route, Dna, Terminal, Settings } from 'lucide-react';
import xLinkLogo from '../assets/X-Link-logo.png';

export type TabId = 'dashboard' | 'profiles' | 'routing' | 'dns' | 'logs' | 'settings';

interface NavRailProps {
  activeTab: TabId;
  onTabChange: (tab: TabId) => void;
  isConnected: boolean;
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

export function NavRail({ activeTab, onTabChange, isConnected, singboxVersion }: NavRailProps) {
  const versionShort = singboxVersion?.match(/v[\d.]+/)?.[0] ?? '—';

  return (
    <aside className="nav-rail">
      {/* Logo */}
      <div className="rail-logo">
        <div className="rail-logo-icon">
          <img src={xLinkLogo} alt="X-Link" style={{ width: '28px', height: '28px', borderRadius: 'var(--r-md)' }} />
        </div>
        <span className="rail-logo-wordmark">X-Link</span>
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
