import React, { useId } from 'react';
import { cn } from '@/lib/utils';

interface FormInputProps extends Omit<React.ComponentProps<'input'>, 'onChange' | 'value'> {
  label?: string;
  value: string | number;
  onChange: (val: string) => void;
  mono?: boolean;
  rows?: number;
}

export function FormInput({ label, value, onChange, placeholder, mono, rows, className, type = 'text', id, ...props }: FormInputProps) {
  const generatedId = useId();
  const inputId = id || generatedId;
  const field = React.createElement('md-outlined-text-field', {
    ...props,
    id: inputId,
    type,
    value: String(value),
    rows,
    label,
    placeholder,
    'aria-label': props['aria-label'] || label || placeholder,
    onInput: (event: React.FormEvent<HTMLElement>) => onChange((event.currentTarget as HTMLElementTagNameMap['md-outlined-text-field']).value),
    className: cn('material-field', mono && 'material-mono', className),
  });

  return <div className="material-field-wrap">{field}</div>;
}
