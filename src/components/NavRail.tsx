import React from 'react';
import { Activity, Layers, Route, Terminal, Settings } from 'lucide-react';
import xLinkLogo from '../assets/X-Link-logo.png';

export type TabId = 'dashboard' | 'profiles' | 'routing' | 'logs' | 'settings';

interface NavRailProps {
  activeTab: TabId;
  onTabChange: (tab: TabId) => void;
}

const NAV_ITEMS: [TabId, React.ComponentType<any>, string][] = [
  ['dashboard', Activity,  'Dashboard'],
  ['profiles',  Layers,    'Config'],
  ['routing',   Route,     'Routing'],
  ['logs',      Terminal,  'Logs'],
  ['settings',  Settings,  'Settings'],
];

export function NavRail({ activeTab, onTabChange }: NavRailProps) {

  return (
    <aside className="nav-rail">
      {/* Logo */}
      <div className="rail-logo">
        <div className="rail-logo-icon">
          <img src={xLinkLogo} alt="X-Link" style={{ width: '28px', height: '28px', borderRadius: 'var(--r-md)', display: 'block' }} />
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

    </aside>
  );
}
