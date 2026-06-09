import React from 'react';

interface ToggleProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'type'> {
}

export const Toggle = React.forwardRef<HTMLInputElement, ToggleProps>(
  ({ className = '', ...props }, ref) => {
    return (
      <label className={`toggle ${className}`.trim()}>
        <input type="checkbox" ref={ref} {...props} />
        <span className="toggle-track" />
        <span className="toggle-thumb" />
      </label>
    );
  }
);
Toggle.displayName = 'Toggle';
