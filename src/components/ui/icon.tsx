import * as React from 'react';
import { cn } from '@/lib/utils';

export interface IconProps extends React.HTMLAttributes<HTMLElement> {
  name?: string;
  size?: number | string;
  className?: string;
  children?: React.ReactNode;
  slot?: string;
}

export function Icon({ name, children, size, className, style, slot, ...props }: IconProps) {
  const iconName = name || (typeof children === 'string' ? children : '');
  return React.createElement(
    'md-icon',
    {
      ...props,
      slot,
      className: cn('material-symbol', className),
      style: {
        fontSize: size ? (typeof size === 'number' ? `${size}px` : size) : undefined,
        width: size ? (typeof size === 'number' ? `${size}px` : size) : undefined,
        height: size ? (typeof size === 'number' ? `${size}px` : size) : undefined,
        ...style,
      },
    },
    iconName
  );
}

export default Icon;
