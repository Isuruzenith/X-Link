import React, { useId } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { cn } from '@/lib/utils';

interface FormInputProps extends Omit<React.ComponentProps<typeof Input>, 'onChange'> {
  label?: string;
  value: string | number;
  onChange: (val: string) => void;
  placeholder?: string;
  mono?: boolean;
}

export function FormInput({ label, value, onChange, placeholder, mono, className, type = "text", id, ...props }: FormInputProps) {
  const generatedId = useId();
  const inputId = id || generatedId;

  return (
    <div className="flex flex-col gap-1.5 w-full">
      {label && (
        <Label htmlFor={inputId} className="text-2xs font-bold text-muted-foreground uppercase tracking-wider cursor-pointer">
          {label}
        </Label>
      )}
      <Input
        id={inputId}
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className={cn(mono && "font-mono text-xs", "h-9 bg-background/50 border-border text-foreground focus-visible:ring-1 focus-visible:ring-ring", className)}
        {...props}
      />
    </div>
  );
}
