import React from 'react';

type BadgeVariant = 'err' | 'ok' | 'warn';

interface BadgeProps {
  variant?: BadgeVariant;
  children: React.ReactNode;
  className?: string;
}

export function Badge({ variant, children, className = '' }: BadgeProps) {
  const cls = `nav-badge ${variant || ''} ${className}`.trim();
  return <span className={cls}>{children}</span>;
}
