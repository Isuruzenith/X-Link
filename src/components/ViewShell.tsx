import type { ReactNode } from 'react';

interface ViewShellProps {
  title: string;
  subtitle?: string;
  actions?: ReactNode;
  children: ReactNode;
}

export function ViewShell({ title, subtitle, actions, children }: ViewShellProps) {
  return (
    <main className="view-shell" id="main-content">
      <header className="view-heading">
        <div className="view-heading-copy">
          <h1>{title}</h1>
          {subtitle && <p>{subtitle}</p>}
        </div>
        {actions && <div className="view-heading-actions">{actions}</div>}
      </header>
      <div className="view-content">{children}</div>
    </main>
  );
}
