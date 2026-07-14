import { Globe, RefreshCw, Check, ShieldAlert, Clipboard, FileUp, Edit3, Zap, CornerDownLeft, Trash2, ArrowRight, Link2, Activity } from 'lucide-react';
import { ViewShell } from '../components/ViewShell';
import { useProfileStore, getCountryCode, type GeoCacheEntry } from '../stores/profileStore';
import { useConnectionStore } from '../stores/connectionStore';
import { useNodeEditorStore } from '../stores/nodeEditorStore';
import type { Profile, ProxyNode, NodeUsageStats } from '../utils/store';
import { useStatsStore } from '../stores/statsStore';
import { Card } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { FormInput } from '@/components/form';
const formatBytes = (bytes: number): string => {
  if (!bytes || bytes === 0) return '0 B';
  const k = 1024, sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
};

const formatRelativeTime = (timestamp?: number): string => {
  if (!timestamp) return '';
  const diff = Date.now() - timestamp;
  const secs = Math.floor(diff / 1000);
  if (secs < 60) return 'just now';
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
};

function ProfileHeaderCard({
  selectedProfile,
  activeProfileId,
  nodes,
  selectedNodeTag,
  isConnected,
  nodeGeoCache,
  latencyResults,
}: {
  selectedProfile: Profile;
  activeProfileId: string | null;
  nodes: ProxyNode[];
  selectedNodeTag: string | null;
  isConnected: boolean;
  nodeGeoCache: Record<string, GeoCacheEntry | 'loading'>;
  latencyResults: Record<string, { latencyMs: number | null; error: string | null }>;
}) {
  const activeNode = selectedNodeTag ? nodes.find((n: ProxyNode) => n.tag === selectedNodeTag) : null;
  const geo = activeNode ? nodeGeoCache[activeNode.server] : null;
  const activeServerCode = geo && geo !== 'loading'
    ? geo.countryCode
    : (activeNode ? getCountryCode(activeNode.tag) : null);
  const activeRegion = geo && geo !== 'loading' && (geo.cityName || geo.regionName)
    ? `${geo.cityName || ''}${geo.cityName && geo.regionName ? ', ' : ''}${geo.regionName || ''}`
    : '';
  const isActiveProfile = selectedProfile.id === activeProfileId;
  const activePing = activeNode && latencyResults[activeNode.tag] ? latencyResults[activeNode.tag] : null;

  return (
    <Card className="p-4 bg-muted/40 border-border mb-4 shrink-0 shadow-none">
      {/* Top row: Profile name + badges */}
      <div className={activeNode ? 'mb-3' : 'mb-0'}>
        <h2 className="text-base font-bold text-foreground mb-1.5">
          {selectedProfile.name}
        </h2>
        <div className="flex gap-1.5 flex-wrap">
          <Badge variant="outline" className="h-5 px-2 text-[10px] font-medium border-border/80 bg-background/50 gap-1 text-muted-foreground">
            <Zap className="size-2.5" />
            <span>{selectedProfile.nodeCount} Nodes</span>
          </Badge>
          {selectedProfile.type === 'subscription' && selectedProfile.subscriptionUrl && (
            <Badge variant="outline" className="h-5 px-2 text-[10px] font-medium border-border/80 bg-background/50 gap-1 text-foreground">
              <Link2 className="size-2.5" />
              <span>Subscription</span>
            </Badge>
          )}
        </div>
      </div>

      {/* Active server details strip */}
      {activeNode && (
        <div className={`flex items-center gap-2.5 p-2 rounded-md border ${isActiveProfile && isConnected ? 'bg-muted/60 border-border/60' : 'bg-background border-border/40'}`}>
          {/* Country flag */}
          <span className="flex items-center justify-center size-6 shrink-0">
            {activeServerCode ? (
              <img
                src={`https://flagcdn.com/w40/${activeServerCode.toLowerCase()}.png`}
                alt={activeServerCode}
                style={{ width: '22px', height: '14px', objectFit: 'cover', borderRadius: '1.5px', boxShadow: '0 1px 2px rgba(0,0,0,0.2)' }}
                onError={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }}
              />
            ) : (
              <Globe className="size-3.5 text-muted-foreground" />
            )}
          </span>

          {/* Server tag */}
          <div className="flex-1 min-w-0">
            <div className="text-[11px] font-bold text-foreground truncate">
              {activeNode.tag}
            </div>
            <span className="text-[9.5px] text-muted-foreground">
              {activeNode.type}{activeRegion ? ` • ${activeRegion}` : ''}
            </span>
          </div>

          {/* Ping / Latency */}
          {activePing && (
            <span className={`text-[10px] font-bold font-mono px-2 py-0.5 rounded-full shrink-0 ${
              activePing.latencyMs !== null
                ? activePing.latencyMs < 200
                  ? 'text-foreground bg-muted border border-border/50'
                  : activePing.latencyMs < 500
                    ? 'text-muted-foreground bg-muted'
                    : 'text-muted-foreground bg-muted/60'
                : 'text-muted-foreground bg-muted/40'
            }`}>
              {activePing.latencyMs !== null ? `${activePing.latencyMs}ms` : 'timeout'}
            </span>
          )}
        </div>
      )}
    </Card>
  );
}

function ServerCard({
  node,
  selectedNodeTag,
  selectNode,
  isConnected,
  selectedProfile,
  activeProfileId,
  openEditor,
  latencyResults,
  activatingNodeTag,
  stats,
}: {
  node: ProxyNode;
  selectedNodeTag: string | null;
  selectNode: (node: ProxyNode) => void;
  isConnected: boolean;
  selectedProfile: Profile;
  activeProfileId: string | null;
  openEditor: (node: ProxyNode) => void;
  latencyResults: Record<string, { latencyMs: number | null; error: string | null }>;
  activatingNodeTag: string | null;
  stats?: NodeUsageStats;
}) {
  const isNodeSelected = selectedNodeTag === node.tag;
  const isNodeActive = isConnected && selectedProfile.id === activeProfileId && isNodeSelected;

  return (
    <Card
      className={`p-3 bg-card border transition-all duration-150 cursor-pointer shadow-sm hover:bg-muted/40 flex flex-row justify-between items-center min-w-0 ${
        isNodeSelected ? 'border-foreground bg-accent/10 ring-1 ring-foreground/20' : 'border-border'
      }`}
      onClick={() => selectNode(node)}
    >
      <div className="flex items-center gap-2.5 overflow-hidden flex-1 min-w-0">
        <div className="flex flex-col gap-1 overflow-hidden flex-1 min-w-0">
          <span className={`text-xs font-bold truncate ${isNodeSelected ? 'text-foreground' : 'text-foreground/90'}`} title={node.tag}>
            {node.tag}
          </span>
          <div className="flex gap-1.5 items-center flex-wrap">
            <Badge variant="outline" className="h-4 px-1.5 text-[9px] font-medium border-border/80 text-muted-foreground">
              {node.type}
            </Badge>
            {isNodeActive ? (
              <Badge className="h-4 px-1.5 text-[8.5px] font-bold rounded-sm bg-foreground text-background">
                ACTIVE
              </Badge>
            ) : (
              activatingNodeTag === node.tag && (
                <div className="flex items-center gap-1.5">
                  <div className="size-1.5 rounded-full bg-muted-foreground animate-pulse" />
                  <span className="text-[8.5px] text-muted-foreground uppercase font-bold">connecting</span>
                </div>
              )
            )}
            {stats && (stats.totalUploadBytes > 0 || stats.totalDownloadBytes > 0) && (
              <span className="text-[9.5px] text-muted-foreground">
                {formatBytes(stats.totalUploadBytes + stats.totalDownloadBytes)}
              </span>
            )}
            {stats?.lastUsedAt && (
              <span className="text-[9.5px] text-muted-foreground">
                • Used {formatRelativeTime(stats.lastUsedAt)}
              </span>
            )}
          </div>
        </div>
      </div>
      <div className="flex items-center gap-2 shrink-0 ml-3">
        {selectedProfile.id === activeProfileId && (
          <Button
            variant="ghost"
            size="icon"
            className="size-7 text-muted-foreground hover:text-foreground rounded"
            onClick={(e) => { e.stopPropagation(); openEditor(node); }}
            title="Edit node configuration"
          >
            <Edit3 className="size-3.5" />
          </Button>
        )}
        {latencyResults[node.tag] && (
          <span
            className={`text-[9.5px] font-bold font-mono px-1.5 py-0.5 rounded-sm ${
              latencyResults[node.tag].latencyMs !== null
                ? latencyResults[node.tag].latencyMs! < 200
                  ? 'text-foreground bg-muted border border-border/50'
                  : latencyResults[node.tag].latencyMs! < 500
                    ? 'text-muted-foreground bg-muted'
                    : 'text-muted-foreground bg-muted/60'
                : 'text-muted-foreground bg-muted/40'
            }`}
            title={latencyResults[node.tag].error || undefined}
          >
            {latencyResults[node.tag].latencyMs !== null
              ? `${latencyResults[node.tag].latencyMs}ms`
              : 'timeout'}
          </span>
        )}
      </div>
    </Card>
  );
}

export function ConfigView() {
  const {
    profiles,
    activeProfileId,
    selectedProfileId,
    nodes,
    selectedNodeTag,
    importName,
    importContent,
    importError,
    importSuccess,
    isImporting,
    latencyResults,
    isTestingLatency,
    setImportName,
    setImportContent,
    pasteClipboard,
    pickFileAndImport,
    importConfig,
    selectNode,
    deleteProfile,
    testAllNodes,
    selectProfile,
    nodeGeoCache,
    activatingNodeTag,
  } = useProfileStore();

  const { isConnected } = useConnectionStore();
  const { openEditor } = useNodeEditorStore();
  const stats = useStatsStore((s) => s.stats);

  const selectedProfile = profiles.find((p) => p.id === selectedProfileId) || null;

  const handleImportSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    importConfig();
  };

  const handleDeleteProfile = (e: React.MouseEvent, profile: Profile) => {
    e.stopPropagation();
    if (confirm(`Delete profile "${profile.name}"? This cannot be undone.`)) {
      deleteProfile(profile.id);
    }
  };

  const getProfileTypeLabel = (type: Profile['type']) => {
    switch (type) {
      case 'subscription': return 'SUB';
      case 'file': return 'FILE';
      case 'manual': return 'MANUAL';
    }
  };

  const getProfileTypeColorClasses = (type: Profile['type']) => {
    switch (type) {
      case 'subscription': return 'bg-muted/80 text-foreground border-border/50';
      case 'file': return 'bg-muted/40 text-muted-foreground border-border/30';
      case 'manual': return 'bg-muted/60 text-muted-foreground border-border/40';
    }
  };

  return (
    <ViewShell title="Profiles" subtitle="Manage proxy configurations and server nodes">
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6 overflow-hidden flex-1 min-h-0">
        {/* Left panel: Profile list + Import */}
        <Card className="p-4 bg-card border-border shadow-sm flex flex-col overflow-hidden h-full">
          {/* Import Form */}
          <div className="shrink-0 p-4 mb-4 bg-muted/40 border border-border/60 rounded-lg">
            <div className="flex justify-between items-center mb-3">
              <h3 className="text-xs font-bold text-foreground flex items-center gap-1.5">
                <FileUp className="size-4 text-muted-foreground" /> Add Profile
              </h3>
              <Button
                variant="outline"
                size="sm"
                className="h-6 gap-1 px-2.5 text-[10px]"
                onClick={pickFileAndImport}
                disabled={isImporting}
                title="Choose config file to import"
              >
                <FileUp className="size-3" /> Choose File
              </Button>
            </div>
            <form onSubmit={handleImportSubmit} className="flex flex-col gap-2">
              <FormInput
                className="h-8"
                value={importName}
                onChange={setImportName}
                placeholder="Profile Name (Optional)"
              />
              <div className="flex gap-2">
                <FormInput
                  className="h-8 flex-1"
                  value={importContent}
                  onChange={setImportContent}
                  placeholder="Paste vless://, ss://, subscription URL…"
                />
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  className="size-8 shrink-0"
                  onClick={pasteClipboard}
                  title="Paste from Clipboard"
                >
                  <Clipboard className="size-3.5" />
                </Button>
                <Button
                  type="submit"
                  variant="default"
                  size="icon"
                  className="size-8 shrink-0"
                  disabled={isImporting || !importContent.trim()}
                  title="Import"
                >
                  {isImporting ? <RefreshCw className="size-3.5 animate-spin" /> : <CornerDownLeft className="size-4" />}
                </Button>
              </div>
              {importError && (
                <div className="bg-destructive/10 text-destructive text-[11px] px-3 py-1.5 rounded border border-destructive/20 flex items-center gap-2">
                  <ShieldAlert className="size-3.5" /> {importError}
                </div>
              )}
              {importSuccess && (
                <div className="bg-muted/80 text-foreground text-[11px] px-3 py-1.5 rounded border border-border/50 flex items-center gap-2">
                  <Check className="size-3.5" /> Profile imported successfully!
                </div>
              )}
            </form>
          </div>

          {/* Profile List */}
          <div className="flex justify-between items-center mb-3 shrink-0 px-0.5">
            <h3 className="text-xs font-bold text-foreground">
              Profiles
              {profiles.length > 0 && <span className="text-muted-foreground font-normal ml-1.5 text-xs">({profiles.length})</span>}
            </h3>
          </div>
          <div className="flex-1 overflow-y-auto pr-1">
            {profiles.length > 0 ? (
              <div className="flex flex-col gap-1.5">
                {profiles.map((profile) => {
                  const isActive = profile.id === activeProfileId;
                  const isSelected = profile.id === selectedProfileId;
                  const typeClasses = getProfileTypeColorClasses(profile.type);
                  return (
                    <div
                      key={profile.id}
                      className={`flex flex-row justify-between items-center p-3 rounded-lg border transition-all duration-150 ${
                        isSelected ? 'border-foreground bg-accent/15' : 'border-border/60 hover:bg-muted/30 cursor-pointer'
                      }`}
                      onClick={() => !isSelected && selectProfile(profile.id)}
                    >
                      <div className="flex items-center gap-3 min-w-0">
                        <div className="size-8 rounded-md bg-muted flex items-center justify-center border border-border/40 text-foreground shrink-0">
                          {profile.type === 'subscription' ? <Link2 className="size-4" /> : <Globe className="size-4" />}
                        </div>
                        <div className="flex flex-col min-w-0">
                          <span className={`text-xs font-bold truncate ${isSelected ? 'text-foreground' : 'text-foreground/90'}`} title={profile.name}>
                            {profile.name}
                          </span>
                          <div className="flex gap-1.5 items-center text-[10px] text-muted-foreground mt-0.5">
                            <Badge variant="outline" className={`h-4 px-1.5 text-[8.5px] font-bold ${typeClasses}`}>
                              {getProfileTypeLabel(profile.type)}
                            </Badge>
                            <span>{profile.nodeCount} nodes</span>
                            <span>•</span>
                            <span>{new Date(profile.lastUpdated).toLocaleDateString()}</span>
                          </div>
                        </div>
                      </div>
                      <div className="flex items-center gap-2.5 shrink-0 ml-3">
                        {isActive && (
                          <div title={isConnected ? "Active & Connected" : "Active"} className="size-2 rounded-full bg-foreground shadow-[0_0_8px_rgba(255,255,255,0.6)]" />
                        )}
                        {!isSelected && !isActive && (
                          <ArrowRight className="size-3.5 text-muted-foreground/60" />
                        )}
                        <Button
                          variant="ghost"
                          size="icon"
                          className="size-7 text-muted-foreground hover:text-destructive hover:bg-destructive/10 rounded"
                          onClick={(e) => handleDeleteProfile(e, profile)}
                          title="Delete profile"
                        >
                          <Trash2 className="size-3.5" />
                        </Button>
                      </div>
                    </div>
                  );
                })}
              </div>
            ) : (
              <div className="flex flex-col items-center justify-center text-muted-foreground h-full min-h-[120px] gap-2">
                <Globe className="size-8 opacity-40 stroke-[1.5]" />
                <p className="text-xs font-bold text-foreground">No Profiles Yet</p>
                <p className="text-[10px] text-center max-w-[220px]">Add a profile above using a subscription link, config file, or pasting node URIs.</p>
              </div>
            )}
          </div>
        </Card>

        {/* Right panel: Node list for active profile */}
        <Card className="p-4 bg-card border-border shadow-sm flex flex-col overflow-hidden h-full">
          {selectedProfile ? (
            <>
              <ProfileHeaderCard
                selectedProfile={selectedProfile}
                activeProfileId={activeProfileId}
                nodes={nodes}
                selectedNodeTag={selectedNodeTag}
                isConnected={isConnected}
                nodeGeoCache={nodeGeoCache}
                latencyResults={latencyResults}
              />

              <div className="flex justify-between items-center mb-3 shrink-0 px-0.5">
                <h3 className="text-xs font-bold text-foreground">Servers</h3>
                <Button
                  variant="outline"
                  size="sm"
                  className="h-7 gap-1 px-3 text-[10px]"
                  onClick={testAllNodes}
                  disabled={isTestingLatency || nodes.length === 0}
                  title="Test TCP latency for all nodes"
                >
                  {isTestingLatency ? <RefreshCw className="size-3 animate-spin" /> : <Activity className="size-3" />}
                  <span>{isTestingLatency ? 'Testing…' : 'Test All'}</span>
                </Button>
              </div>

              <div className="flex-1 overflow-y-auto pr-1">
                {nodes.length > 0 ? (
                  <div className="grid grid-cols-1 gap-2">
                    {nodes.map((node, i) => (
                      <ServerCard
                        key={i}
                        node={node}
                        selectedNodeTag={selectedNodeTag}
                        selectNode={selectNode}
                        isConnected={isConnected}
                        selectedProfile={selectedProfile}
                        activeProfileId={activeProfileId}
                        openEditor={openEditor}
                        latencyResults={latencyResults}
                        activatingNodeTag={activatingNodeTag}
                        stats={stats[node.tag]}
                      />
                    ))}
                  </div>
                ) : (
                  <div className="flex items-center justify-center h-24 text-muted-foreground">
                    <p className="text-xs">No customizable servers found.</p>
                  </div>
                )}
              </div>
            </>
          ) : (
            <div className="flex flex-col items-center justify-center flex-1 text-muted-foreground gap-3">
              <div className="size-16 rounded-full bg-muted flex items-center justify-center border border-border/40 shadow-sm">
                <Globe className="size-7 text-foreground" />
              </div>
              <div className="flex flex-col items-center gap-1">
                <p className="text-sm font-bold text-foreground">No Profile Selected</p>
                <p className="text-[11px] max-w-[240px] text-center leading-relaxed">
                  Add a profile on the left to see its available servers here.
                </p>
              </div>
            </div>
          )}
        </Card>
      </div>
    </ViewShell>
  );
}
