import * as React from 'react';
import { cn } from '@/lib/utils';

interface SwitchProps {
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  disabled?: boolean;
  className?: string;
  id?: string;
  'aria-label'?: string;
}

function Switch({ checked, onCheckedChange, className, ...props }: SwitchProps) {
  return React.createElement('md-switch', {
    ...props,
    selected: checked,
    className: cn('material-switch', className),
    onInput: (event: React.FormEvent<HTMLElement>) => {
      onCheckedChange((event.currentTarget as HTMLElementTagNameMap['md-switch']).selected);
    },
  });
}

export { Switch };
