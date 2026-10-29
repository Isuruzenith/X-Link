import * as React from "react"
import { cn } from "@/lib/utils"

export interface BadgeProps extends React.ComponentProps<"span"> {
  variant?: "default" | "secondary" | "destructive" | "outline";
}

function Badge({
  className,
  variant = "default",
  ...props
}: BadgeProps) {
  const variantStyles = {
    default: "bg-[var(--md-sys-color-primary)] text-[var(--md-sys-color-on-primary)]",
    secondary: "bg-[var(--md-sys-color-secondary-container)] text-[var(--md-sys-color-on-secondary-container)]",
    destructive: "bg-[var(--md-sys-color-error-container)] text-[var(--md-sys-color-on-error-container)]",
    outline: "border border-[var(--md-sys-color-outline-variant)] text-[var(--md-sys-color-on-surface-variant)]",
  };

  return (
    <span
      data-slot="badge"
      data-variant={variant}
      className={cn(
        "inline-flex h-5 w-fit shrink-0 items-center justify-center gap-1 overflow-hidden rounded-full px-2 py-0.5 text-[11px] font-semibold whitespace-nowrap transition-colors",
        variantStyles[variant],
        className
      )}
      {...props}
    />
  );
}

export { Badge }

