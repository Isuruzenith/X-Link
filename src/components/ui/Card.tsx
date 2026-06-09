import React from 'react';

interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  flush?: boolean;
  accent?: boolean;
  children: React.ReactNode;
}

export function Card({ flush, accent, className = '', children, ...props }: CardProps) {
  const cls = `card ${flush ? 'flush' : ''} ${accent ? 'accent' : ''} ${className}`.trim();
  return (
    <div className={cls} {...props}>
      {children}
    </div>
  );
}

export function CardDivider() {
  return <div className="card-divider" />;
}
