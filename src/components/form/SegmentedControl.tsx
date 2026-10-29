import React from 'react';
import { Button } from '@/components/ui/button';
import { Icon } from '@/components/ui/icon';
import { cn } from '@/lib/utils';

export interface SegmentOption<T extends string> {
  value: T;
  label: string;
  icon?: string | React.ComponentType<{ className?: string }>;
}

interface SegmentedControlProps<T extends string> {
  value: T;
  onChange: (val: T) => void;
  options: SegmentOption<T>[];
  className?: string;
}

export function SegmentedControl<T extends string>({ value, onChange, options, className }: SegmentedControlProps<T>) {
  return (
    <div className={cn("material-segments flex items-center bg-muted/60 p-1 rounded-2xl shrink-0 overflow-x-auto whitespace-nowrap [scrollbar-width:none] [&::-webkit-scrollbar]:hidden", className)}>
      {options.map((opt) => {
        const iconElement = typeof opt.icon === 'string'
          ? <Icon name={opt.icon} size={14} className="shrink-0" />
          : opt.icon
            ? React.createElement(opt.icon, { className: 'size-3 shrink-0' })
            : null;
        const isActive = value === opt.value;
        return (
          <Button
            key={opt.value}
            type="button"
            onClick={() => onChange(opt.value)}
            variant="ghost"
            aria-pressed={isActive}
            className={cn('material-segment', isActive && 'is-active')}
          >
            {iconElement}
            <span>{opt.label}</span>
          </Button>
        );
      })}
    </div>
  );
}

