import { X, CheckCircle, AlertTriangle, AlertCircle, Info } from 'lucide-react';
import { useToastStore, type ToastType } from '../stores/toastStore';
import { Button } from '@/components/ui/button';

const TOAST_ICONS: Record<ToastType, typeof CheckCircle> = {
  success: CheckCircle,
  error: AlertCircle,
  warning: AlertTriangle,
  info: Info,
};

const TOAST_CLASSES: Record<ToastType, { border: string; icon: string }> = {
  success: { border: 'border-border/60', icon: 'text-foreground' },
  error:   { border: 'border-destructive/30 bg-destructive/5', icon: 'text-destructive' },
  warning: { border: 'border-border/40', icon: 'text-foreground/80' },
  info:    { border: 'border-border/40', icon: 'text-foreground/80' },
};

export function ToastContainer() {
  const { toasts, removeToast } = useToastStore();

  if (toasts.length === 0) return null;

  return (
    <>
      {/* Bottom progressive blur backdrop scrim */}
      <div className="fixed bottom-0 inset-x-0 h-28 bg-gradient-to-t from-background/80 to-transparent pointer-events-none backdrop-blur-[2px] z-40" />

      {/* Centered Toast List */}
      <div className="fixed bottom-6 left-1/2 -translate-x-1/2 z-50 flex flex-col gap-2 w-[360px] max-w-[calc(100vw-32px)] pointer-events-none">
        {toasts.map((toast) => {
          const Icon = TOAST_ICONS[toast.type];
          const classes = TOAST_CLASSES[toast.type];

          return (
            <div
              key={toast.id}
              className={`flex items-start gap-3 p-3.5 px-4 rounded-xl bg-card/95 border shadow-lg backdrop-blur-md pointer-events-auto transition-all animate-in fade-in slide-in-from-bottom-5 duration-300 ${classes.border}`}
            >
              <Icon className={`size-4 shrink-0 mt-0.5 ${classes.icon}`} />
              
              <div className="flex flex-col gap-0.5 flex-1 min-w-0">
                {toast.title && (
                  <span className="text-xs font-bold text-foreground truncate">
                    {toast.title}
                  </span>
                )}
                <span className="text-[11px] text-muted-foreground leading-relaxed">
                  {toast.message}
                </span>
              </div>

              <Button
                variant="ghost"
                size="icon"
                className="size-6 text-muted-foreground hover:text-foreground hover:bg-muted/40 rounded shrink-0 -mt-0.5 -mr-1"
                onClick={() => removeToast(toast.id)}
              >
                <X className="size-3.5" />
              </Button>
            </div>
          );
        })}
      </div>
    </>
  );
}
