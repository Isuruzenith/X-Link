import { Switch } from '@/components/ui/switch';
import { cn } from '@/lib/utils';

interface FormSwitchProps {
  title: string;
  desc?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  className?: string;
  disabled?: boolean;
}

export function FormSwitch({ title, desc, checked, onChange, className, disabled }: FormSwitchProps) {
  return (
    <div className={cn("flex items-center justify-between gap-4 py-2.5 border-b border-border/20 last:border-0", disabled && "opacity-50 pointer-events-none", className)}>
      <div className="flex flex-col gap-0.5 min-w-0 flex-1">
        <span className="text-xs font-semibold text-foreground leading-none">{title}</span>
        {desc && <span className="text-2xs text-muted-foreground leading-normal mt-1">{desc}</span>}
      </div>
      <Switch
        checked={checked}
        onCheckedChange={onChange}
        disabled={disabled}
      />
    </div>
  );
}

export function FormSwitchRow({ title, desc, checked, onChange, className, disabled }: FormSwitchProps) {
  return (
    <div className={cn("flex items-center justify-between gap-3 p-2.5 px-3 bg-muted/40 border border-border/40 rounded-lg", disabled && "opacity-50 pointer-events-none", className)}>
      <div className="flex flex-col gap-0.5 min-w-0 flex-1">
        <span className="text-xs font-semibold text-foreground truncate">{title}</span>
        {desc && <span className="text-2xs text-muted-foreground truncate">{desc}</span>}
      </div>
      <Switch
        checked={checked}
        onCheckedChange={onChange}
        disabled={disabled}
        className="shrink-0"
      />
    </div>
  );
}
