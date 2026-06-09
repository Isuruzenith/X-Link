import React from 'react';
import { Info, CheckCircle2, AlertTriangle, ShieldAlert } from 'lucide-react';

type AlertVariant = 'info' | 'success' | 'warn' | 'error';

interface AlertProps {
  variant?: AlertVariant;
  children: React.ReactNode;
  className?: string;
}

const ICONS = {
  info: <Info size={16} />,
  success: <CheckCircle2 size={16} />,
  warn: <AlertTriangle size={16} />,
  error: <ShieldAlert size={16} />,
};

export function Alert({ variant = 'info', className = '', children }: AlertProps) {
  const cls = `alert ${variant} ${className}`.trim();
  return (
    <div className={cls}>
      <div style={{ marginTop: '1px' }}>{ICONS[variant]}</div>
      <div>{children}</div>
    </div>
  );
}
