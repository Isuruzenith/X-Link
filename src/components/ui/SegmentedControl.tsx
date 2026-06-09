import React from 'react';

export interface SegmentedControlItem {
  id: string;
  label: React.ReactNode;
  icon?: React.ReactNode;
}

interface SegmentedControlProps {
  items: SegmentedControlItem[];
  activeId: string;
  onChange: (id: string) => void;
  className?: string;
}

export function SegmentedControl({ items, activeId, onChange, className = '' }: SegmentedControlProps) {
  return (
    <div className={`seg-control ${className}`.trim()}>
      {items.map((item) => (
        <div
          key={item.id}
          className={`seg-item ${activeId === item.id ? 'active' : ''}`}
          onClick={() => onChange(item.id)}
        >
          {item.icon}
          {item.label}
        </div>
      ))}
    </div>
  );
}
