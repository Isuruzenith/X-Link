import { useRef, useEffect, useState, useDeferredValue } from 'react';
import { Terminal, Copy, Trash2, Search } from 'lucide-react';
import { ViewShell } from '../components/ViewShell';
import { useLogStore } from '../stores/logStore';
import { useToastStore } from '../stores/toastStore';
import { Card } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { FormInput, SegmentedControl } from '@/components/form';

export function LogsView() {
  const { logs, autoScroll, setAutoScroll, clearLogs, copyLogs } = useLogStore();
  const containerRef = useRef<HTMLDivElement>(null);
  const [filterText, setFilterText] = useState('');
  const deferredFilterText = useDeferredValue(filterText);
  const [levelFilter, setLevelFilter] = useState<'all' | 'info' | 'warn' | 'error' | 'system'>('all');

  const handleScroll = () => {
    const container = containerRef.current;
    if (!container) return;

    // Check if user is within 40px of bottom
    const isAtBottom = container.scrollHeight - container.scrollTop - container.clientHeight < 40;
    if (isAtBottom && !autoScroll) {
      setAutoScroll(true);
    } else if (!isAtBottom && autoScroll) {
      setAutoScroll(false);
    }
  };

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    if (autoScroll) {
      container.scrollTop = container.scrollHeight;
    }
  }, [logs, autoScroll]);

  const filtered = logs.filter((l) => {
    if (levelFilter !== 'all' && l.type !== levelFilter) return false;
    if (deferredFilterText && !l.text.toLowerCase().includes(deferredFilterText.toLowerCase())) return false;
    return true;
  });

  const handleCopy = async () => {
    await copyLogs();
    useToastStore.getState().addToast('success', 'All logs copied to clipboard.', 'Logs Copied');
  };

  // Syntax highlighting for log tags like [DNS], [TUN], etc.
  const renderHighlightedText = (text: string) => {
    const regex = /(\[[^\]]+\])/g;
    const parts = text.split(regex);
    return parts.map((part, idx) => {
      if (part.startsWith('[') && part.endsWith(']')) {
        const tagText = part.slice(1, -1);
        const lowerTag = tagText.toLowerCase();
        
        let classes = 'text-muted-foreground bg-muted border-border/40';
        
        if (lowerTag.includes('warn')) {
          classes = 'text-foreground bg-transparent border-dashed border-border';
        } else if (lowerTag.includes('err') || lowerTag.includes('fail')) {
          classes = 'text-background bg-foreground border-foreground';
        }
        
        return (
          <span
            key={idx}
            className={`inline-block px-1.5 py-0.5 rounded text-[9.5px] font-bold uppercase tracking-wider border mr-1.5 leading-none ${classes}`}
          >
            {tagText}
          </span>
        );
      }
      return <span key={idx}>{part}</span>;
    });
  };

  const getLevelLabel = (type: string) => {
    if (type === 'error') return 'ERR';
    if (type === 'warn')  return 'WARN';
    if (type === 'system') return 'SYS';
    return 'INFO';
  };

  return (
    <ViewShell
      title="Console Logs"
      subtitle="Real-time core proxy logs and system events output"
      actions={
        <div className="flex items-center gap-2 select-none">
          <Button variant="outline" size="sm" className="h-8 gap-1 px-3 text-xs font-semibold" onClick={handleCopy}>
            <Copy className="size-3.5" /> Copy All
          </Button>
          <Button variant="outline" size="sm" className="h-8 gap-1 px-3 text-xs font-semibold" onClick={clearLogs}>
            <Trash2 className="size-3.5" /> Clear
          </Button>
          <div className="flex items-center gap-2 border-l border-border/80 pl-3.5 ml-1 h-5">
            <Switch id="autoscroll-toggle" checked={autoScroll} onCheckedChange={setAutoScroll} />
            <label htmlFor="autoscroll-toggle" className="text-xs font-bold text-muted-foreground cursor-pointer select-none">
              Autoscroll
            </label>
          </div>
        </div>
      }
    >
      <div className="flex flex-col gap-[10px] overflow-hidden h-full w-full min-h-0">

        {/* Filter bar */}
        <div className="flex items-center gap-3 shrink-0 select-none">
          <div className="relative flex-1 min-w-[140px] max-w-[320px]">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground z-10" />
            <FormInput
              className="pl-9 h-8"
              placeholder="Search console output…"
              value={filterText}
              onChange={setFilterText}
            />
          </div>
          
          <SegmentedControl
            value={levelFilter}
            onChange={(val) => setLevelFilter(val as 'all' | 'info' | 'warn' | 'error' | 'system')}
            options={[
              { label: 'All', value: 'all' },
              { label: 'Info', value: 'info' },
              { label: 'Warn', value: 'warn' },
              { label: 'Errors', value: 'error' },
              { label: 'System', value: 'system' }
            ]}
          />
          
          <span className="text-xs font-semibold text-muted-foreground ml-auto hidden sm:inline">
            Showing {filtered.length} of {logs.length} entries
          </span>
        </div>

        {/* Terminal block */}
        <Card className="flex-1 p-0 overflow-hidden bg-card border-border shadow-sm flex flex-col min-h-0">
          <div
            ref={containerRef}
            onScroll={handleScroll}
            className="flex-1 w-full min-h-0 overflow-y-auto p-4 font-mono text-[11.5px] leading-relaxed"
          >
            {filtered.length > 0 ? (
              filtered.map((log, i) => (
                <div
                  key={i}
                  className="flex items-start py-0.5 px-1.5 rounded mb-0.5 hover:bg-muted/30 transition-colors font-normal text-foreground/90"
                >
                  {/* Timestamp */}
                  <span className="text-muted-foreground/60 mr-3.5 select-none text-[10px] shrink-0 font-normal">
                    {log.timestamp}
                  </span>

                  {/* Level tag */}
                  <span className={`w-9 shrink-0 text-[9px] font-bold tracking-wider select-none ${
                    log.type === 'error' ? 'text-foreground font-black' :
                    log.type === 'warn'  ? 'text-muted-foreground' :
                    log.type === 'system' ? 'text-muted-foreground/80' : 'text-muted-foreground/60'
                  }`}>
                    {getLevelLabel(log.type)}
                  </span>

                  {/* Log content */}
                  <span className={`flex-1 break-all font-normal text-[11px] ${
                    log.type === 'error' ? 'text-destructive/90 font-medium' : 'text-foreground/90'
                  }`}>
                    {renderHighlightedText(log.text)}
                  </span>
                </div>
              ))
            ) : (
              <div className="flex flex-col items-center justify-center h-full min-h-[300px] text-muted-foreground gap-2.5 select-none animate-in fade-in duration-200">
                <Terminal className="size-9 opacity-60 stroke-[1.2]" />
                <span className="text-xs font-bold text-foreground">
                  {logs.length > 0 ? 'No logs match the current filters.' : 'Waiting for core process console output…'}
                </span>
              </div>
            )}
          </div>
        </Card>
      </div>
    </ViewShell>
  );
}
