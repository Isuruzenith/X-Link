import { X, CheckCircle, AlertTriangle, AlertCircle, Info } from 'lucide-react';
import { useToastStore, type ToastType } from '../stores/toastStore';

const TOAST_ICONS: Record<ToastType, typeof CheckCircle> = {
  success: CheckCircle,
  error: AlertCircle,
  warning: AlertTriangle,
  info: Info,
};

const TOAST_GLOWS: Record<ToastType, string> = {
  success: 'var(--status-ok-glow)',
  error: 'var(--status-err-glow)',
  warning: 'var(--status-warn-dim)',
  info: 'var(--status-info-dim)',
};

const TOAST_COLORS: Record<ToastType, { border: string; icon: string }> = {
  success: { border: 'var(--status-ok-dim)', icon: 'var(--status-ok)' },
  error:   { border: 'var(--status-err-dim)', icon: 'var(--status-err)' },
  warning: { border: 'var(--status-warn-dim)', icon: 'var(--status-warn)' },
  info:    { border: 'var(--status-info-dim)', icon: 'var(--status-info)' },
};

export function ToastContainer() {
  const { toasts, removeToast } = useToastStore();

  if (toasts.length === 0) return null;

  return (
    <>
      {/* Bottom progressive blur backdrop */}
      <div className="bottom-blur-scrim" />

      {/* Centered Toast List */}
      <div style={{
        position: 'fixed',
        bottom: '24px',
        left: '50%',
        transform: 'translateX(-50%)',
        zIndex: 'var(--z-toast)' as React.CSSProperties['zIndex'],
        display: 'flex',
        flexDirection: 'column',
        gap: '10px',
        width: '360px',
        maxWidth: 'calc(100vw - 32px)',
        pointerEvents: 'none',
      }}>
        {toasts.map((toast) => {
          const Icon = TOAST_ICONS[toast.type];
          const colors = TOAST_COLORS[toast.type];
          const glow = TOAST_GLOWS[toast.type];

          return (
            <div
              key={toast.id}
              style={{
                display: 'flex',
                alignItems: 'flex-start',
                gap: '12px',
                padding: '13px 18px',
                borderRadius: '12px',
                background: 'rgba(25, 26, 32, 0.65)',
                border: '1px solid rgba(255, 255, 255, 0.12)',
                backdropFilter: 'blur(24px) saturate(185%)',
                boxShadow: `0 16px 36px rgba(0, 0, 0, 0.4), inset 0 1px 0 rgba(255, 255, 255, 0.16), 0 0 10px ${glow}`,
                animation: 'toast-slide-up 0.3s cubic-bezier(0.16, 1, 0.3, 1) forwards',
                pointerEvents: 'auto',
              }}
            >
              <Icon size={16} style={{ color: colors.icon, flexShrink: 0, marginTop: '2px' }} />
              
              <div style={{ display: 'flex', flexDirection: 'column', gap: '3px', flex: 1 }}>
                {toast.title && (
                  <span style={{ fontSize: '13px', fontWeight: 650, color: 'var(--text-high)' }}>
                    {toast.title}
                  </span>
                )}
                <span style={{ fontSize: '11.5px', color: 'var(--text-med)', lineHeight: '1.4' }}>
                  {toast.message}
                </span>
              </div>

              <button
                onClick={() => removeToast(toast.id)}
                style={{
                  background: 'none',
                  border: 'none',
                  cursor: 'pointer',
                  color: 'var(--text-low)',
                  padding: '2px',
                  borderRadius: '4px',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  flexShrink: 0,
                  transition: 'color 0.15s, background 0.15s',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.color = 'var(--text-high)';
                  e.currentTarget.style.background = 'rgba(255, 255, 255, 0.05)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.color = 'var(--text-low)';
                  e.currentTarget.style.background = 'none';
                }}
              >
                <X size={13} />
              </button>
            </div>
          );
        })}
      </div>
    </>
  );
}
