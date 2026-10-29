import { useEffect, useState, useRef, useDeferredValue, memo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ask } from '@tauri-apps/plugin-dialog';
import { Network, X, ShieldAlert, ArrowUpDown, RefreshCw } from 'lucide-react';
import { ViewShell } from '../components/ViewShell';
import { useConnectionStore } from '../stores/connectionStore';
import { useToastStore } from '../stores/toastStore';
import { Card } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { ScrollArea } from '@/components/ui/scroll-area';
import { FormInput, SegmentedControl } from '@/components/form';
import { cn } from '@/lib/utils';

interface ConnectionMetadata {
  network: string;
  type: string;
  sourceIP: string;
  sourcePort: string;
  destinationIP: string;
  destinationPort: string;
  host: string;
}

interface ConnectionEntry {
  id: string;
  metadata: ConnectionMetadata;
  upload: number;
  download: number;
  start: string; // ISO string
  chains: string[];
  rule: string;
  // Computed local speeds
  upSpeed?: number;
  downSpeed?: number;
}

const formatBytes = (bytes: number): string => {
  if (bytes <= 0) return '0 B';
  const k = 1024, sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
};

const formatSpeed = (bps: number): string => {
  if (bps <= 0) return '0 B/s';
  const k = 1024, sizes = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
  const i = Math.floor(Math.log(bps) / Math.log(k));
  return parseFloat((bps / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
};

const formatDuration = (startIso: string): string => {
  try {
    const elapsed = Date.now() - new Date(startIso).getTime();
    const seconds = Math.floor(elapsed / 1000);
    if (seconds < 60) return `${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m ${seconds % 60}s`;
    const hours = Math.floor(minutes / 60);
    return `${hours}h ${minutes % 60}m`;
  } catch {
    return '0s';
  }
};

const ConnectionRow = memo(({
  conn,
  isSelected,
  onSelect,
  onKill,
  isKilling,
}: {
  conn: ConnectionEntry;
  isSelected: boolean;
  onSelect: () => void;
  onKill: (id: string, label: string) => void;
  isKilling: boolean;
}) => {
  const host = conn.metadata.host || conn.metadata.destinationIP;
  const port = conn.metadata.destinationPort;
  const isDirect = conn.chains.includes('direct');
  const chainDisplay = conn.chains.join(' › ');

  return (
    <TableRow
      className={`text-xs transition-colors border-b border-border/30 ${
        isSelected ? 'bg-muted/60 hover:bg-muted/70' : 'hover:bg-muted/20'
      }`}
    >
      {/* Target Address */}
      <TableCell className="py-2.5 px-4 max-w-[280px] font-normal">
        <div className="flex flex-col gap-0.5">
          <button type="button" className="font-bold text-foreground truncate text-left max-w-full" title={`${host}:${port}`} onClick={onSelect} aria-pressed={isSelected}>
            {host}
            <span className="text-muted-foreground font-normal font-mono">:{port}</span>
          </button>
          <div className="flex gap-1.5 items-center mt-0.5 select-none">
            <Badge variant="outline" className="h-4 px-1.5 text-[8.5px] font-mono border-border bg-background/50 font-normal uppercase">
              {conn.metadata.network}
            </Badge>
            <span className="text-[9.5px] text-muted-foreground">
              type: {conn.metadata.type}
            </span>
          </div>
        </div>
      </TableCell>

      {/* Matching Route Rule */}
      <TableCell className="py-2.5 px-4 font-normal">
        <div className="flex flex-col gap-0.5">
          <div className="flex items-center select-none">
            <Badge variant="outline" className={cn('text-2xs font-bold uppercase tracking-wide', 
              isDirect 
                ? 'bg-transparent text-muted-foreground border-border'
                : 'bg-primary text-primary-foreground border-primary'
            )}>
              {chainDisplay}
            </Badge>
          </div>
          {conn.rule && (
            <div className="text-[9.5px] text-muted-foreground font-mono mt-0.5" title={conn.rule}>
              rule: {conn.rule}
            </div>
          )}
        </div>
      </TableCell>

      {/* Real-time calculated speeds */}
      <TableCell className="py-2.5 px-4 font-mono font-normal">
        <div className="flex flex-col gap-0.5 text-[10.5px]">
          <span className="text-foreground">↓ {formatSpeed(conn.downSpeed || 0)}</span>
          <span className="text-muted-foreground">↑ {formatSpeed(conn.upSpeed || 0)}</span>
        </div>
      </TableCell>

      {/* Total bytes stats */}
      <TableCell className="py-2.5 px-4 font-mono font-normal">
        <div className="flex flex-col gap-0.5 text-[10.5px] text-muted-foreground">
          <span>↓ {formatBytes(conn.download)}</span>
          <span>↑ {formatBytes(conn.upload)}</span>
        </div>
      </TableCell>

      {/* Connection lifetime */}
      <TableCell className="py-2.5 px-4 text-foreground text-[11px] font-medium">
        {formatDuration(conn.start)}
      </TableCell>

      {/* Termination */}
      <TableCell className="py-2.5 px-4 text-center select-none" onClick={(e) => e.stopPropagation()}>
        <Button
          variant="ghost"
          size="icon"
          className="size-7 text-muted-foreground hover:text-destructive hover:bg-destructive/10 rounded"
          onClick={() => onKill(conn.id, `${host}:${port}`)}
          aria-label={`Close connection to ${host}:${port}`}
          title="Force close connection"
          disabled={isKilling}
        >
          {isKilling ? (
            <RefreshCw className="size-3.5 animate-spin" />
          ) : (
            <X className="size-3.5" />
          )}
        </Button>
      </TableCell>
    </TableRow>
  );
});
ConnectionRow.displayName = 'ConnectionRow';

export function ConnectionsView() {
  const { isConnected } = useConnectionStore();
  const [connections, setConnections] = useState<ConnectionEntry[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const deferredSearchQuery = useDeferredValue(searchQuery);
  const [filterType, setFilterType] = useState<'all' | 'proxy' | 'direct'>('all');
  const [sortBy, setSortBy] = useState<'host' | 'speed' | 'bytes' | 'time'>('bytes');
  const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('desc');
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [killingIds, setKillingIds] = useState<Record<string, boolean>>({});

  const prevBytesRef = useRef<Record<string, { upload: number; download: number; time: number }>>({});

  useEffect(() => {
    if (!isConnected) return;

    const fetchConnections = async () => {
      try {
        const res = await invoke<{ connections: ConnectionEntry[] }>('get_active_connections');
        const list = res?.connections || [];
        const now = Date.now();

        // Calculate per-connection speed
        const updatedList = list.map((conn) => {
          const prev = prevBytesRef.current[conn.id];
          let upSpeed = 0;
          let downSpeed = 0;

          if (prev) {
            const timeDiff = (now - prev.time) / 1000;
            if (timeDiff > 0) {
              upSpeed = Math.max(0, (conn.upload - prev.upload) / timeDiff);
              downSpeed = Math.max(0, (conn.download - prev.download) / timeDiff);
            }
          }

          return {
            ...conn,
            upSpeed,
            downSpeed,
          };
        });

        // Update prev reference
        const nextBytes: Record<string, { upload: number; download: number; time: number }> = {};
        updatedList.forEach((c) => {
          nextBytes[c.id] = { upload: c.upload, download: c.download, time: now };
        });
        prevBytesRef.current = nextBytes;

        setConnections(updatedList);
      } catch {
        // fail silently
      }
    };

    fetchConnections();
    const interval = setInterval(fetchConnections, 1000);
    return () => clearInterval(interval);
  }, [isConnected]);

  const handleKill = async (id: string, label: string) => {
    setKillingIds((prev) => ({ ...prev, [id]: true }));
    try {
      await invoke('close_connection', { id });
      useToastStore.getState().addToast('info', `Closed connection to ${label}`);
      setConnections((prev) => prev.filter((c) => c.id !== id));
      if (selectedId === id) {
        setSelectedId(null);
      }
    } catch {
      useToastStore.getState().addToast('error', 'Failed to close connection');
    } finally {
      setKillingIds((prev) => {
        const copy = { ...prev };
        delete copy[id];
        return copy;
      });
    }
  };

  const handleKillAll = async () => {
    const confirmed = await ask('Close all active network connections?', {
      title: 'X-Link',
      kind: 'warning',
    });
    if (confirmed) {
      try {
        await invoke('close_all_connections');
        useToastStore.getState().addToast('info', 'All connections closed');
        setConnections([]);
        setSelectedId(null);
      } catch {
        useToastStore.getState().addToast('error', 'Failed to close connections');
      }
    }
  };

  // Filter connections
  const visibleConnections = isConnected ? connections : [];

  const filtered = visibleConnections.filter((c) => {
    const host = c.metadata.host || c.metadata.destinationIP;
    const rule = c.rule || '';
    const ip = c.metadata.destinationIP || '';
    
    // Search query match
    if (deferredSearchQuery.trim()) {
      const q = deferredSearchQuery.toLowerCase();
      const matchSearch =
        host.toLowerCase().includes(q) ||
        rule.toLowerCase().includes(q) ||
        ip.toLowerCase().includes(q);
      if (!matchSearch) return false;
    }

    // Type filter match
    const isDirect = c.chains.includes('direct');
    if (filterType === 'proxy' && isDirect) return false;
    if (filterType === 'direct' && !isDirect) return false;

    return true;
  });

  // Sorting
  const sorted = [...filtered].sort((a, b) => {
    let valA: string | number = 0;
    let valB: string | number = 0;

    if (sortBy === 'host') {
      valA = a.metadata.host || a.metadata.destinationIP || '';
      valB = b.metadata.host || b.metadata.destinationIP || '';
    } else if (sortBy === 'speed') {
      valA = (a.upSpeed || 0) + (a.downSpeed || 0);
      valB = (b.upSpeed || 0) + (b.downSpeed || 0);
    } else if (sortBy === 'bytes') {
      valA = a.upload + a.download;
      valB = b.upload + b.download;
    } else if (sortBy === 'time') {
      valA = new Date(a.start).getTime();
      valB = new Date(b.start).getTime();
    }

    if (valA < valB) return sortOrder === 'asc' ? -1 : 1;
    if (valA > valB) return sortOrder === 'asc' ? 1 : -1;
    return 0;
  });

  const toggleSort = (field: typeof sortBy) => {
    if (sortBy === field) {
      setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc');
    } else {
      setSortBy(field);
      setSortOrder('desc');
    }
  };

  return (
    <ViewShell
      title="Connections"
      subtitle="See active destinations, routing decisions and transferred data"
      actions={
        <div className="flex items-center gap-2">
          {visibleConnections.length > 0 && (
            <Button
              variant="destructive"
              size="sm"
              className="h-8 gap-1 px-3 text-xs font-semibold"
              onClick={handleKillAll}
            >
              <ShieldAlert className="size-3.5" /> Close All
            </Button>
          )}
        </div>
      }
    >
      <div className="utility-view">
        
        {/* Controls row */}
        <div className="utility-toolbar select-none">
          <div className="utility-search">
            <FormInput
              type="search"
              label="Search connections"
              placeholder="Host, IP or rule…"
              value={searchQuery}
              onChange={setSearchQuery}
            />
          </div>
          
          <SegmentedControl
            value={filterType}
            onChange={(val) => setFilterType(val as 'all' | 'proxy' | 'direct')}
            options={[
              { value: 'all', label: 'All Tunnels' },
              { value: 'proxy', label: 'Proxy Tunnels' },
              { value: 'direct', label: 'Direct Tunnels' },
            ]}
          />

          <span className="text-xs font-semibold text-muted-foreground ml-auto hidden sm:inline-block min-w-[20ch] text-right tabular-nums">
            {sorted.length} / {visibleConnections.length} active sockets
          </span>
        </div>
 
        {/* Connections table panel */}
        <Card className="flex-1 p-0 overflow-hidden bg-card flex flex-col min-h-[300px]">
          <ScrollArea className="utility-table flex-1 w-full">
            {sorted.length > 0 ? (
              <Table>
                <caption className="sr-only">Active connections, routes, speeds and transferred data</caption>
                <TableHeader>
                  <TableRow className="hover:bg-transparent bg-muted/40 border-b border-border/80 text-[11px] font-semibold text-muted-foreground select-none">
                    <TableHead className="h-9 px-4">
                      <button type="button" className={cn("flex items-center justify-between gap-1 w-full min-w-[120px] pr-2 text-left", sortBy === 'host' ? 'text-foreground font-bold' : 'text-muted-foreground hover:text-foreground')} onClick={() => toggleSort('host')} aria-label={`Sort by destination ${sortBy === 'host' && sortOrder === 'desc' ? 'ascending' : 'descending'}`}>
                        <span>Destination</span>
                        <ArrowUpDown className={cn("size-3 shrink-0 transition-colors", sortBy === 'host' ? 'text-foreground' : 'text-muted-foreground/30')} />
                      </button>
                    </TableHead>
                    <TableHead className="h-9 px-4">
                      <div className="resize-x overflow-hidden min-w-[150px] max-w-[400px] pr-2">
                        Network / Matched Rule
                      </div>
                    </TableHead>
                    <TableHead className="h-9 px-4">
                      <button type="button" className={cn("flex items-center justify-between gap-1 w-full min-w-[100px] pr-2 text-left", sortBy === 'speed' ? 'text-foreground font-bold' : 'text-muted-foreground hover:text-foreground')} onClick={() => toggleSort('speed')} aria-label={`Sort by live speed ${sortBy === 'speed' && sortOrder === 'desc' ? 'ascending' : 'descending'}`}>
                        <span>Live speed</span>
                        <ArrowUpDown className={cn("size-3 shrink-0 transition-colors", sortBy === 'speed' ? 'text-foreground' : 'text-muted-foreground/30')} />
                      </button>
                    </TableHead>
                    <TableHead className="h-9 px-4">
                      <button type="button" className={cn("flex items-center justify-between gap-1 w-full min-w-[100px] pr-2 text-left", sortBy === 'bytes' ? 'text-foreground font-bold' : 'text-muted-foreground hover:text-foreground')} onClick={() => toggleSort('bytes')} aria-label={`Sort by total data ${sortBy === 'bytes' && sortOrder === 'desc' ? 'ascending' : 'descending'}`}>
                        <span>Total data</span>
                        <ArrowUpDown className={cn("size-3 shrink-0 transition-colors", sortBy === 'bytes' ? 'text-foreground' : 'text-muted-foreground/30')} />
                      </button>
                    </TableHead>
                    <TableHead className="h-9 px-4">
                      <button type="button" className={cn("flex items-center justify-between gap-1 w-full min-w-[90px] pr-2 text-left", sortBy === 'time' ? 'text-foreground font-bold' : 'text-muted-foreground hover:text-foreground')} onClick={() => toggleSort('time')} aria-label={`Sort by uptime ${sortBy === 'time' && sortOrder === 'desc' ? 'ascending' : 'descending'}`}>
                        <span>Uptime</span>
                        <ArrowUpDown className={cn("size-3 shrink-0 transition-colors", sortBy === 'time' ? 'text-foreground' : 'text-muted-foreground/30')} />
                      </button>
                    </TableHead>
                    <TableHead className="h-9 px-4 w-14 text-center">
                      <div className="w-full text-center">Kill</div>
                    </TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {sorted.map((conn) => (
                    <ConnectionRow
                      key={conn.id}
                      conn={conn}
                      isSelected={selectedId === conn.id}
                      onSelect={() => setSelectedId(selectedId === conn.id ? null : conn.id)}
                      onKill={handleKill}
                      isKilling={!!killingIds[conn.id]}
                    />
                  ))}
                </TableBody>
              </Table>
            ) : (
              <div className="flex flex-col items-center justify-center h-full min-h-[300px] text-muted-foreground p-10 gap-3 select-none">
                <Network className="size-10 opacity-60 stroke-[1.2]" />
                <span className="text-xs font-bold text-foreground">
                  {!isConnected ? 'Proxy service is disconnected.' : 'No active network connections found.'}
                </span>
                {isConnected && searchQuery && (
                  <span className="text-[10px]">Try modifying your filter query.</span>
                )}
              </div>
            )}
          </ScrollArea>
        </Card>
      </div>
    </ViewShell>
  );
}
