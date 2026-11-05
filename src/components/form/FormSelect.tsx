import * as React from 'react';
import { cn } from '@/lib/utils';

export interface FormSelectOption {
  value: string;
  label: string;
}

interface FormSelectProps {
  label?: string;
  value: string;
  onChange: (val: string) => void;
  options: FormSelectOption[];
  placeholder?: string;
  className?: string;
  disabled?: boolean;
  'aria-invalid'?: boolean;
  'aria-describedby'?: string;
}

export function FormSelect({
  label,
  value,
  onChange,
  options,
  placeholder,
  className,
  disabled,
  'aria-invalid': ariaInvalid,
  'aria-describedby': ariaDescribedBy,
}: FormSelectProps) {
  const selectRef = React.useRef<HTMLElementTagNameMap['md-outlined-select']>(null);

  React.useEffect(() => {
    if (selectRef.current && selectRef.current.value !== value) {
      selectRef.current.value = value;
    }
  }, [value]);

  const select = React.createElement(
    'md-outlined-select',
    {
      ref: selectRef,
      label,
      value,
      'aria-label': label || placeholder || 'Select option',
      'aria-invalid': ariaInvalid === undefined ? undefined : String(ariaInvalid),
      'aria-describedby': ariaDescribedBy,
      error: ariaInvalid || undefined,
      disabled: disabled || undefined,
      className: cn('material-field material-select', className),
      onChange: (event: React.FormEvent<HTMLElementTagNameMap['md-outlined-select']>) => {
        onChange(event.currentTarget.value);
      },
    },
    options.map((opt) =>
      React.createElement(
        'md-select-option',
        {
          key: opt.value,
          value: opt.value,
          selected: opt.value === value || undefined,
        },
        React.createElement('div', { slot: 'headline' }, opt.label)
      )
    )
  );

  return <div className="material-field-wrap">{select}</div>;
}

