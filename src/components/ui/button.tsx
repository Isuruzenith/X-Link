import * as React from 'react';
import { cn } from '@/lib/utils';
import { Icon } from './icon';

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
  const slottedChildren = iconOnly ? children : React.Children.map(children, (child) => {
    if (React.isValidElement(child) && (
      child.type === Icon || child.type === 'md-icon' ||
      (child.props as Record<string, unknown>)['data-icon'] === 'true' ||
      (child.props as Record<string, unknown>)['data-icon'] === true
    )) {
      const icon = child as React.ReactElement<{ slot?: string }>;
      return React.cloneElement(icon, { slot: icon.props.slot ?? 'icon' });
    }
    return child;
  });
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
    onClick: (e: React.MouseEvent<HTMLButtonElement>) => {
      if (props.disabled) {
        e.preventDefault();
        return;
      }
      if (type === 'submit') {
        const form = (e.currentTarget as HTMLElement).closest('form');
        if (form) {
          form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
        }
      }
      props.onClick?.(e);
    },
  }, slottedChildren);
}

export { Button };
