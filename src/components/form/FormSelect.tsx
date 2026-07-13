import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Label } from '@/components/ui/label';
import { cn } from '@/lib/utils';

export interface FormSelectOption {
  value: string;
  label: string;
}

interface FormSelectProps {
  label?: string;
  value: string;
  onChange: (val: string) => void;
  options: FormSelectOption[];
  placeholder?: string;
  className?: string;
  disabled?: boolean;
}

export function FormSelect({ label, value, onChange, options, placeholder, className, disabled }: FormSelectProps) {
  return (
    <div className="flex flex-col gap-1.5 w-full">
      {label && <Label className="text-2xs font-bold text-muted-foreground uppercase tracking-wider">{label}</Label>}
      <Select value={value} onValueChange={onChange} disabled={disabled}>
        <SelectTrigger className={cn("h-9 bg-background/50 border-border text-foreground text-xs focus:ring-1 focus:ring-ring justify-between", className)}>
          <SelectValue placeholder={placeholder} />
        </SelectTrigger>
        <SelectContent>
          {options.map((opt) => (
            <SelectItem key={opt.value} value={opt.value} className="text-xs">
              {opt.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
