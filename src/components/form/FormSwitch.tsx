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

function SwitchRow({ title, desc, checked, onChange, className, disabled }: FormSwitchProps) {
  return (
    <div className={cn('material-setting-row', disabled && 'is-disabled', className)}>
      <div className="material-setting-copy">
        <span className="material-setting-title">{title}</span>
        {desc && <span className="material-setting-desc">{desc}</span>}
      </div>
      <Switch checked={checked} onCheckedChange={onChange} aria-label={title} disabled={disabled} className="shrink-0" />
    </div>
  );
}

export const FormSwitch = SwitchRow;
export const FormSwitchRow = SwitchRow;
