import * as React from 'react';
import { cn } from '@/lib/utils';

export interface CheckboxProps {
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  disabled?: boolean;
  className?: string;
  id?: string;
  'aria-label'?: string;
}

export function Checkbox({
  checked,
  onCheckedChange,
  disabled,
  className,
  id,
  'aria-label': ariaLabel,
  ...props
}: CheckboxProps) {
  const checkboxRef = React.useRef<HTMLElementTagNameMap['md-checkbox']>(null);

  React.useEffect(() => {
    if (checkboxRef.current && checkboxRef.current.checked !== checked) {
      checkboxRef.current.checked = checked;
    }
  }, [checked]);

  return React.createElement('md-checkbox', {
    ...props,
    ref: checkboxRef,
    id,
    checked: checked || undefined,
    disabled: disabled || undefined,
    'touch-target': 'wrapper',
    'aria-label': ariaLabel,
    className: cn('material-checkbox', className),
    onChange: (e: React.FormEvent<HTMLElementTagNameMap['md-checkbox']>) => {
      onCheckedChange(e.currentTarget.checked);
    },
  });
}
