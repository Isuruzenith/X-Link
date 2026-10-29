import * as React from 'react';
import { cn } from '@/lib/utils';

type ButtonProps = React.ComponentProps<'button'> & {
  variant?: 'default' | 'outline' | 'secondary' | 'ghost' | 'destructive' | 'link';
  size?: 'default' | 'xs' | 'sm' | 'lg' | 'icon' | 'icon-xs' | 'icon-sm' | 'icon-lg';
};

const TAGS = {
  default: 'md-filled-button',
  secondary: 'md-filled-tonal-button',
  outline: 'md-outlined-button',
  ghost: 'md-text-button',
  link: 'md-text-button',
  destructive: 'md-filled-tonal-button',
} as const;

function Button({
  className, variant = 'default', size = 'default', type = 'button',
  children, title, 'aria-label': ariaLabel, ...props
}: ButtonProps) {
  const iconOnly = size.startsWith('icon');
  const tag = iconOnly ? 'md-icon-button' : TAGS[variant];
  // React 19 serialises boolean values on custom elements as "" for true, which is
  // not a valid ARIA token, so stringify state attributes explicitly.
  const aria = Object.fromEntries(
    (['aria-expanded', 'aria-pressed', 'aria-selected', 'aria-checked'] as const)
      .filter((name) => typeof props[name] === 'boolean')
      .map((name) => [name, String(props[name])]),
  );
  return React.createElement(tag, {
    ...props,
    ...aria,
    type,
    title,
    'aria-label': ariaLabel || (iconOnly ? title : undefined),
    style: iconOnly ? {
      '--md-icon-button-state-layer-height': '100%',
      '--md-icon-button-state-layer-width': '100%',
      ...props.style,
    } as React.CSSProperties : props.style,
    className: cn('material-action', iconOnly && 'material-icon-action', `material-action-${variant}`, className),
  }, children);
}

export { Button };
