import React from 'react';

export interface Option {
  value: string;
  label: string;
}

interface SelectProps extends React.SelectHTMLAttributes<HTMLSelectElement> {
  options: Option[];
  mono?: boolean;
  error?: boolean;
}

export const Select = React.forwardRef<HTMLSelectElement, SelectProps>(
  ({ options, mono, error, className = '', ...props }, ref) => {
    const classes = [
      'field',
      mono ? 'mono' : '',
      error ? 'error' : '',
      className
    ].filter(Boolean).join(' ');

    return (
      <select ref={ref} className={classes} {...props}>
        {options.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
    );
  }
);
Select.displayName = 'Select';
