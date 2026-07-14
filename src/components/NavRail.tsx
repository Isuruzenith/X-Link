import React from 'react';
import { Activity, Layers, Route, Terminal, Settings, Network } from 'lucide-react';
import xLinkLogo from '../assets/X-Link-logo.png';
import { useConnectionStore } from '../stores/connectionStore';
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
  const { connectionStatus } = useConnectionStore();

  return (
    <aside className="w-[48px] hover:w-[180px] transition-all duration-200 ease-out h-full flex flex-col bg-sidebar border-r border-sidebar-border py-3 overflow-hidden group/sidebar z-20 shrink-0 select-none relative">
      {/* Logo */}
      <div className="px-2 mb-5 shrink-0">
        <div className="flex items-center justify-center group-hover/sidebar:justify-start gap-0 group-hover/sidebar:gap-3 h-10 px-0 group-hover/sidebar:px-3 rounded-md whitespace-nowrap transition-all duration-150 w-full">
          <div className="size-7 rounded-md flex items-center justify-center shrink-0 bg-sidebar-accent border border-sidebar-border shadow-sm relative">
            <img src={xLinkLogo} alt="X-Link" className="size-full rounded-md block" />
            <div 
              className={cn(
                "absolute -bottom-0.5 -right-0.5 size-2 rounded-full border border-sidebar",
                connectionStatus === 'connected' ? 'bg-emerald-500' :
                connectionStatus === 'connecting' ? 'bg-amber-500 animate-pulse' :
                connectionStatus === 'disconnected' ? 'bg-zinc-500' : 'bg-muted-foreground'
              )}
              title={`Connection status: ${connectionStatus}`}
              aria-label={`Connection status: ${connectionStatus}`}
              role="status"
            />
          </div>
          <span className="text-sm font-bold tracking-wide text-sidebar-foreground opacity-0 group-hover/sidebar:opacity-100 group-hover/sidebar:translate-x-0 -translate-x-1 transition-all duration-200 flex-none group-hover/sidebar:flex-grow group-hover/sidebar:flex-1 w-0 overflow-hidden group-hover/sidebar:w-auto group-hover/sidebar:overflow-visible">
            X-Link
          </span>
        </div>
      </div>
 
      {/* Navigation */}
      <nav className="flex flex-col gap-0.5 px-2 flex-1" role="navigation" aria-label="Main navigation">
        {NAV_ITEMS.map(([tab, Icon, label]) => (
          <button
            key={tab}
            type="button"
            aria-current={activeTab === tab ? 'page' : undefined}
            className={cn(
              'flex items-center justify-center group-hover/sidebar:justify-start gap-0 group-hover/sidebar:gap-3 h-[38px] px-0 group-hover/sidebar:px-3 rounded-md cursor-pointer relative whitespace-nowrap transition-all duration-150 border border-transparent text-sidebar-foreground/75 hover:bg-sidebar-accent hover:text-sidebar-accent-foreground w-full text-left',
              activeTab === tab && 'bg-sidebar-accent text-sidebar-accent-foreground border-sidebar-border/30 font-medium'
            )}
            onClick={() => onTabChange(tab)}
          >
            <div
              className={cn(
                'absolute left-[-8px] top-[20%] bottom-[20%] w-[3px] rounded-r-full bg-sidebar-foreground transition-opacity',
                activeTab === tab ? 'opacity-100' : 'opacity-0'
              )}
            />
            <div className="size-7 flex items-center justify-center shrink-0">
              <Icon className={cn('size-[18px] shrink-0 transition-colors', activeTab === tab && 'text-sidebar-accent-foreground')} />
            </div>
            <span className="text-xs opacity-0 group-hover/sidebar:opacity-100 group-hover/sidebar:translate-x-0 -translate-x-2 transition-all duration-200 flex-none group-hover/sidebar:flex-grow group-hover/sidebar:flex-1 w-0 overflow-hidden group-hover/sidebar:w-auto group-hover/sidebar:overflow-visible">
              {label}
            </span>
          </button>
        ))}
      </nav>
    </aside>
  );
}
