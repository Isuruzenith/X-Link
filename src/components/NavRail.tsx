import React from 'react';
import { Activity, Layers, Route, Terminal, Settings, Network } from 'lucide-react';
import xLinkLogo from '../assets/X-Link-logo.png';
import { cn } from '@/lib/utils';

export type TabId = 'dashboard' | 'profiles' | 'routing' | 'logs' | 'connections' | 'settings';

interface NavRailProps {
  activeTab: TabId;
  onTabChange: (tab: TabId) => void;
}

const NAV_ITEMS: [TabId, React.ComponentType<{ className?: string; size?: number }>, string][] = [
  ['dashboard', Activity,  'Dashboard'],
  ['profiles',  Layers,    'Config'],
  ['routing',   Route,     'Routing'],
  ['connections', Network,  'Connections'],
  ['logs',      Terminal,  'Logs'],
  ['settings',  Settings,  'Settings'],
];

export function NavRail({ activeTab, onTabChange }: NavRailProps) {
  return (
    <aside className="w-[56px] hover:w-[220px] transition-all duration-200 ease-out h-full flex flex-col bg-sidebar border-r border-sidebar-border py-3 overflow-hidden group/sidebar z-20 shrink-0 select-none relative">
      {/* Logo */}
      <div className="flex items-center gap-3 px-3.5 py-3 mb-5 h-10 overflow-hidden whitespace-nowrap">
        <div className="size-7 rounded-md flex items-center justify-center shrink-0 overflow-hidden bg-sidebar-accent border border-sidebar-border shadow-sm">
          <img src={xLinkLogo} alt="X-Link" className="size-7 rounded-md block" />
        </div>
        <span className="text-sm font-bold tracking-wide text-sidebar-foreground opacity-0 group-hover/sidebar:opacity-100 group-hover/sidebar:translate-x-0 -translate-x-1 transition-all duration-200">
          X-Link
        </span>
      </div>

      {/* Navigation */}
      <nav className="flex flex-col gap-0.5 px-2 flex-1" aria-label="Main navigation">
        {NAV_ITEMS.map(([tab, Icon, label]) => (
          <button
            key={tab}
            type="button"
            aria-current={activeTab === tab ? 'page' : undefined}
            className={cn(
              'flex items-center justify-center group-hover/sidebar:justify-start gap-0 group-hover/sidebar:gap-3 h-[38px] px-0 group-hover/sidebar:px-3 rounded-md cursor-pointer relative overflow-hidden whitespace-nowrap transition-all duration-150 border border-transparent text-sidebar-foreground/75 hover:bg-sidebar-accent hover:text-sidebar-accent-foreground w-full text-left',
              activeTab === tab && 'bg-sidebar-accent text-sidebar-accent-foreground border-sidebar-border/30 font-medium'
            )}
            onClick={() => onTabChange(tab)}
          >
            <div
              className={cn(
                'absolute left-0 top-[25%] bottom-[25%] w-[3px] rounded-r-full bg-sidebar-foreground/70 transition-opacity',
                activeTab === tab ? 'opacity-100' : 'opacity-0'
              )}
            />
            <Icon className={cn('size-[18px] shrink-0 transition-colors', activeTab === tab && 'text-sidebar-accent-foreground')} />
            <span className="text-xs opacity-0 group-hover/sidebar:opacity-100 group-hover/sidebar:translate-x-0 -translate-x-2 transition-all duration-200 flex-none group-hover/sidebar:flex-grow group-hover/sidebar:flex-1 w-0 overflow-hidden group-hover/sidebar:w-auto group-hover/sidebar:overflow-visible">
              {label}
            </span>
          </button>
        ))}
      </nav>
    </aside>
  );
}
