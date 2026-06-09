import React from 'react';

interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  mono?: boolean;
  error?: boolean;
}

export const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ mono, error, className = '', ...props }, ref) => {
    const classes = [
      'field',
      mono ? 'mono' : '',
      error ? 'error' : '',
      className
    ].filter(Boolean).join(' ');

    return <input ref={ref} className={classes} {...props} />;
  }
);
Input.displayName = 'Input';
