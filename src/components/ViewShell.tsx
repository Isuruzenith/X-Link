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
      <div className="flex items-center justify-between px-6 py-3.5 border-b border-border bg-card shrink-0 select-none">
        <div className="flex flex-col gap-0.5 min-w-0 pr-4">
          <span className="text-lg font-semibold text-foreground tracking-tight truncate">{title}</span>
          {subtitle && <span className="text-xs text-muted-foreground truncate">{subtitle}</span>}
        </div>
        {actions && <div className="flex items-center gap-2 shrink-0">{actions}</div>}
      </div>
      <div className="flex-1 overflow-hidden bg-background/50 flex flex-col px-6 py-4 min-h-0">
        {children}
      </div>
    </div>
  );
}
