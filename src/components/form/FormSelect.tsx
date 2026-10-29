import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useId } from 'react';
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

const EMPTY_OPTION = '__xlink-empty-option__';

export function FormSelect({ label, value, onChange, options, placeholder, className, disabled }: FormSelectProps) {
  const labelId = useId();
  const hasEmptyOption = options.some((option) => option.value === '');
  return (
    <div className="flex flex-col gap-1.5 w-full">
      {label && <span id={labelId} className="text-xs font-medium text-muted-foreground">{label}</span>}
      <Select
        value={hasEmptyOption && value === '' ? EMPTY_OPTION : value}
        onValueChange={(selected) => onChange(selected === EMPTY_OPTION ? '' : selected)}
        disabled={disabled}
      >
        <SelectTrigger aria-labelledby={label ? labelId : undefined} aria-label={!label ? placeholder || 'Select option' : undefined} className={cn("material-select h-11 bg-background/50 border-border text-foreground text-xs focus:ring-1 focus:ring-ring justify-between", disabled && "opacity-50 cursor-not-allowed", className)}>
          <SelectValue placeholder={placeholder} />
        </SelectTrigger>
        <SelectContent position="popper">
          {options.map((opt) => (
            <SelectItem key={opt.value} value={opt.value === '' ? EMPTY_OPTION : opt.value} className="text-xs">
              {opt.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
