import React from 'react';

type ChipVariant = 'default' | 'accent' | 'success' | 'warn' | 'error' | 'purple';

interface ChipProps extends React.HTMLAttributes<HTMLSpanElement> {
  variant?: ChipVariant;
  children: React.ReactNode;
}

export function Chip({ variant = 'default', className = '', children, ...props }: ChipProps) {
  const cls = `chip ${variant} ${className}`.trim();
  return (
    <span className={cls} {...props}>
      {children}
    </span>
  );
}
