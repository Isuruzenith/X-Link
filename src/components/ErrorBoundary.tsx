import React from 'react';
import { AlertCircle, RefreshCw } from 'lucide-react';

interface Props {
  children: React.ReactNode;
  fallbackTitle?: string;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends React.Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error('ErrorBoundary caught:', error, errorInfo);
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null });
  };

  render() {
    if (this.state.hasError) {
      return (
        <div style={{
          display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
          padding: '40px 24px', textAlign: 'center', color: 'var(--text-low)',
          height: '100%', minHeight: '200px',
        }}>
          <div style={{
            width: '56px', height: '56px', borderRadius: '50%',
            background: 'var(--status-err-dim)', border: '1px solid rgba(239,68,68,0.2)',
            display: 'flex', alignItems: 'center', justifyContent: 'center', marginBottom: '16px',
          }}>
            <AlertCircle size={24} style={{ color: 'var(--status-err)' }} />
          </div>
          <h3 style={{ fontSize: '15px', fontWeight: 600, color: 'var(--text-high)', marginBottom: '8px' }}>
            {this.props.fallbackTitle || 'Something went wrong'}
          </h3>
          <p style={{ fontSize: '12px', maxWidth: '340px', lineHeight: '1.5', marginBottom: '16px' }}>
            {this.state.error?.message || 'An unexpected error occurred in this component.'}
          </p>
          <button
            className="btn primary sm"
            onClick={this.handleReset}
            style={{ display: 'flex', alignItems: 'center', gap: '6px' }}
          >
            <RefreshCw size={13} /> Try Again
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}
