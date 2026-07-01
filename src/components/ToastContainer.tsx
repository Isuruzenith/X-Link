import { X, CheckCircle, AlertTriangle, AlertCircle, Info } from 'lucide-react';
import { useToastStore, type ToastType } from '../stores/toastStore';

const TOAST_ICONS: Record<ToastType, typeof CheckCircle> = {
  success: CheckCircle,
  error: AlertCircle,
  warning: AlertTriangle,
  info: Info,
};

const TOAST_COLORS: Record<ToastType, { bg: string; border: string; icon: string }> = {
  success: { bg: 'var(--status-ok-dim)', border: 'rgba(46,213,115,0.2)', icon: 'var(--status-ok)' },
  error:   { bg: 'var(--status-err-dim)', border: 'rgba(239,68,68,0.2)', icon: 'var(--status-err)' },
  warning: { bg: 'var(--status-warn-dim)', border: 'rgba(245,158,11,0.2)', icon: 'var(--status-warn)' },
  info:    { bg: 'var(--accent-primary-dim)', border: 'var(--border-accent-dim)', icon: 'var(--accent-primary)' },
};

export function ToastContainer() {
  const { toasts, removeToast } = useToastStore();

  if (toasts.length === 0) return null;

  return (
    <div style={{
      position: 'fixed',
      bottom: '20px',
      right: '20px',
      zIndex: 'var(--z-toast)' as any,
      display: 'flex',
      flexDirection: 'column',
      gap: '8px',
      maxWidth: '380px',
      pointerEvents: 'none',
    }}>
      {toasts.map((toast) => {
        const Icon = TOAST_ICONS[toast.type];
        const colors = TOAST_COLORS[toast.type];
        return (
          <div
            key={toast.id}
            style={{
              display: 'flex',
              alignItems: 'flex-start',
              gap: '10px',
              padding: '12px 14px',
              borderRadius: 'var(--r-md)',
              background: colors.bg,
              border: `1px solid ${colors.border}`,
              backdropFilter: 'blur(16px)',
              boxShadow: '0 4px 20px rgba(0,0,0,0.3)',
              animation: 'toast-slide-in 0.25s ease-out',
              pointerEvents: 'auto',
            }}
          >
            <Icon size={16} style={{ color: colors.icon, flexShrink: 0, marginTop: '1px' }} />
            <span style={{ fontSize: '12px', color: 'var(--text-high)', lineHeight: '1.4', flex: 1 }}>
              {toast.message}
            </span>
            <button
              onClick={() => removeToast(toast.id)}
              style={{
                background: 'none', border: 'none', cursor: 'pointer',
                color: 'var(--text-low)', padding: '0', flexShrink: 0,
              }}
            >
              <X size={14} />
            </button>
          </div>
        );
      })}
    </div>
  );
}
