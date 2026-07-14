import React from 'react';

interface ViewShellProps {
  title: string;
  subtitle?: string;
  actions?: React.ReactNode;
  children: React.ReactNode;
}

export function ViewShell({ title, subtitle, actions, children }: ViewShellProps) {
  return (
    <div className="flex flex-col h-full overflow-hidden min-h-0">
      <div 
        className="flex items-center justify-between border-b border-border bg-card shrink-0 select-none"
        style={{ paddingLeft: '10px', paddingRight: '10px', paddingTop: '6px', paddingBottom: '6px' }}
      >
        <div className="flex flex-col min-w-0 pr-4">
          <span className="text-sm font-bold text-foreground tracking-tight truncate">{title}</span>
          {subtitle && <span className="text-[10px] text-muted-foreground truncate leading-normal">{subtitle}</span>}
        </div>
        {actions && <div className="flex items-center gap-2 shrink-0">{actions}</div>}
      </div>
      <div 
        className="flex-1 overflow-y-auto bg-background/50 flex flex-col min-h-0"
        style={{ padding: '10px' }}
      >
        {children}
      </div>
    </div>
  );
}
