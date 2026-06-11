import React from 'react';

interface ViewShellProps {
  title: string;
  subtitle?: string;
  actions?: React.ReactNode;
  children: React.ReactNode;
}

export function ViewShell({ title, subtitle, actions, children }: ViewShellProps) {
  return (
    <>
      <div className="view-topbar">
        <div className="view-topbar-left">
          <span className="view-topbar-title">{title}</span>
          {subtitle && <span className="view-topbar-sub">{subtitle}</span>}
        </div>
        {actions && <div className="view-topbar-right">{actions}</div>}
      </div>
      <div className="view-body">
        {children}
      </div>
    </>
  );
}
