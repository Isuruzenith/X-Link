import React from 'react';
import { AlertCircle, RefreshCw, RotateCw } from 'lucide-react';
import { Button } from '@/components/ui/button';

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
        <div className="flex flex-col items-center justify-center p-10 text-center text-muted-foreground h-full min-h-[200px]">
          <div className="size-14 rounded-full bg-destructive/10 border border-destructive/25 flex items-center justify-center mb-4 shadow-sm">
            <AlertCircle className="size-6 text-destructive" />
          </div>
          <h3 className="text-sm font-bold text-foreground mb-2">
            {this.props.fallbackTitle || 'Something went wrong'}
          </h3>
          <p className="text-xs max-w-[340px] leading-relaxed mb-4 text-muted-foreground/80">
            {this.state.error?.message || 'An unexpected error occurred in this component.'}
          </p>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={this.handleReset}
              className="h-8.5 gap-1.5 px-4 text-xs font-semibold"
            >
              <RefreshCw className="size-3.5" /> Try Again
            </Button>
            <Button
              variant="default"
              size="sm"
              onClick={() => window.location.reload()}
              className="h-8.5 gap-1.5 px-4 text-xs font-semibold"
            >
              <RotateCw className="size-3.5" /> Reload App
            </Button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
