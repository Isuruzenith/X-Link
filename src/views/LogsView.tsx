import React, { useEffect, useState, useDeferredValue, useMemo } from 'react';
import { Icon } from '@/components/ui/icon';
import { List, useListRef, type RowComponentProps } from 'react-window';
import { ViewShell } from '../components/ViewShell';
import { useLogStore, type LogEntry } from '../stores/logStore';
import { useToastStore } from '../stores/toastStore';
import { Card } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { FormInput, SegmentedControl } from '@/components/form';

const LOG_ROW_HEIGHT = 30;

function getLevelLabel(type: string): string {
  if (type === 'error') return 'ERR';
  if (type === 'warn')  return 'WARN';
  if (type === 'system') return 'SYS';
  return 'INFO';
}

function stripRedundantLevelTag(text: string, type: string): string {
  if (type === 'system') return text.replace(/^\[(?:system|sys)\]\s*/i, '');
  if (type === 'info') return text.replace(/^\[(?:info)\]\s*/i, '');
  if (type === 'warn') return text.replace(/^\[(?:warn|warning)\]\s*/i, '');
  if (type === 'error') return text.replace(/^\[(?:error|err)\]\s*/i, '');
  return text;
}

// Syntax highlighting for log tags like [DNS], [TUN], etc.
// Off-screen rows are not rendered by react-window, so highlighting is only evaluated for visible rows.
function renderHighlightedText(text: string) {
  const regex = /(\[[^\]]+\])/g;
  const parts = text.split(regex);
  return parts.map((part, idx) => {
    if (part.startsWith('[') && part.endsWith(']')) {
      const tagText = part.slice(1, -1);
      const lowerTag = tagText.toLowerCase();

      let classes = 'text-foreground/75 bg-muted border-border/40';

      if (lowerTag.includes('warn')) {
        classes = 'text-amber-700 bg-amber-50/50 border-amber-200 dark:text-amber-400 dark:bg-amber-950/20 dark:border-amber-900/50';
      } else if (lowerTag.includes('err') || lowerTag.includes('fail')) {
        classes = 'text-destructive bg-destructive/10 border-destructive/20';
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
}

interface LogRowProps {
  items: LogEntry[];
}

function LogRow({
  index,
  style,
  items,
}: RowComponentProps<LogRowProps>): React.ReactElement | null {
  const log = items[index];
  if (!log) return null;

  const cleanText = stripRedundantLevelTag(log.text, log.type);

  return (
    <div
      style={style}
      className="flex items-center px-2 py-0.5 rounded hover:bg-muted/30 transition-colors font-mono text-[11.5px] font-normal text-foreground/90 select-text gap-2 min-w-0"
    >
      {/* Timestamp */}
      <span className="text-muted-foreground/60 select-none text-[10px] shrink-0 font-normal w-14 truncate" title={log.timestamp}>
        {log.timestamp}
      </span>

      {/* Level label */}
      <span className={`w-9 shrink-0 text-[9px] font-bold tracking-wider select-none uppercase ${
        log.type === 'error' ? 'text-destructive font-black' :
        log.type === 'warn'  ? 'text-amber-600 dark:text-amber-500' :
        log.type === 'system' ? 'text-blue-600 dark:text-blue-400' : 'text-foreground/70'
      }`}>
        {getLevelLabel(log.type)}
      </span>

      {/* Log content */}
      <span
        className={`flex-1 truncate font-normal text-[11px] min-w-0 ${
          log.type === 'error' ? 'text-destructive/90 font-medium' : 'text-foreground/90'
        }`}
        title={cleanText}
      >
        {renderHighlightedText(cleanText)}
      </span>
    </div>
  );
}

export function LogsView() {
  const { logs, autoScroll, setAutoScroll, clearLogs, copyLogs } = useLogStore();
  const listRef = useListRef(null);
  const [filterText, setFilterText] = useState('');
  const deferredFilterText = useDeferredValue(filterText);
  const [levelFilter, setLevelFilter] = useState<'all' | 'info' | 'warn' | 'error' | 'system'>('all');

  const filtered = useMemo(() => {
    return logs.filter((l) => {
      if (levelFilter !== 'all' && l.type !== levelFilter) return false;
      if (deferredFilterText && !l.text.toLowerCase().includes(deferredFilterText.toLowerCase())) return false;
      return true;
    });
  }, [logs, levelFilter, deferredFilterText]);

  const rowProps = useMemo<LogRowProps>(() => ({ items: filtered }), [filtered]);

  const handleScroll = (e: React.UIEvent<HTMLDivElement>) => {
    const container = e.currentTarget;
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
    if (!autoScroll || filtered.length === 0) return;

    const scrollToBottom = () => {
      const el = listRef.current?.element;
      if (el) {
        el.scrollTop = el.scrollHeight;
      } else {
        listRef.current?.scrollToRow({
          index: filtered.length - 1,
          align: 'end',
          behavior: 'auto',
        });
      }
    };

    scrollToBottom();
    const raf = requestAnimationFrame(scrollToBottom);
    return () => cancelAnimationFrame(raf);
  }, [filtered.length, autoScroll, listRef]);

  const handleCopy = async () => {
    try {
      await copyLogs();
      useToastStore.getState().addToast('success', 'All logs copied to clipboard.', 'Logs Copied');
    } catch {
      useToastStore.getState().addToast('error', 'Clipboard access failed. Try again.', 'Copy Failed');
    }
  };

  return (
    <ViewShell
      title="Logs"
      subtitle="Follow proxy activity and system events as they happen"
      actions={
        <div className="utility-toolbar select-none">
          <Button variant="outline" size="sm" className="h-8 gap-1 px-3 text-xs font-semibold" onClick={handleCopy}>
            <Icon name="content_copy" size={14} /> Copy All
          </Button>
          <Button variant="outline" size="sm" className="h-8 gap-1 px-3 text-xs font-semibold" onClick={clearLogs}>
            <Icon name="delete" size={14} /> Clear
          </Button>
          <div className="flex items-center gap-2 border-l border-border/80 pl-3.5 ml-1 h-5">
            <Switch id="autoscroll-toggle" checked={autoScroll} onCheckedChange={setAutoScroll} aria-label="Autoscroll logs" />
            <label htmlFor="autoscroll-toggle" className="text-xs font-bold text-muted-foreground cursor-pointer select-none">
              Autoscroll
            </label>
          </div>
        </div>
      }
    >
      <div className="utility-view utility-log">

        {/* Filter bar */}
        <div className="utility-toolbar utility-log-filters select-none">
          <div className="utility-search">
            <FormInput
              type="search"
              label="Search logs"
              placeholder="Find a message…"
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
        <Card className="utility-log-panel flex-1 p-0 overflow-hidden bg-card flex flex-col min-h-[300px]">
          {filtered.length > 0 ? (
            <div className="flex-1 w-full min-h-0 p-3 font-mono text-[11.5px]" role="region" aria-label="Proxy activity logs">
              <List
                listRef={listRef}
                rowCount={filtered.length}
                rowHeight={LOG_ROW_HEIGHT}
                rowComponent={LogRow}
                rowProps={rowProps}
                overscanCount={6}
                onScroll={handleScroll}
                style={{ height: '100%', width: '100%' }}
              />
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center h-full min-h-[300px] text-muted-foreground gap-2.5 select-none animate-in fade-in duration-200">
              <Icon name="terminal" size={36} className="opacity-60" />
              <span className="text-xs font-bold text-foreground">
                {logs.length > 0 ? 'No logs match the current filters.' : 'Waiting for core process console output…'}
              </span>
            </div>
          )}
        </Card>
      </div>
    </ViewShell>
  );
}
