import { useCallback, useEffect, useRef, useState } from 'react';
import { Icon } from '@/components/ui/icon';
import { ask } from '@tauri-apps/plugin-dialog';
import { ViewShell } from '../components/ViewShell';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { FormInput } from '@/components/form';
import { useProfileStore, getCountryCode } from '../stores/profileStore';
import { useConnectionStore } from '../stores/connectionStore';
import { useSettingsStore } from '../stores/settingsStore';
import { useNodeEditorStore } from '../stores/nodeEditorStore';
import { useStatsStore } from '../stores/statsStore';
import type { Profile, ProxyNode } from '../utils/store';

const formatBytes = (bytes: number): string => {
  if (!bytes) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  return `${parseFloat((bytes / 1024 ** index).toFixed(1))} ${units[index]}`;
};

export function ConfigView() {
  const {
    profiles, activeProfileId, selectedProfileId, nodes, selectedNodeTag,
    importName, importContent, importError, importSuccess, isImporting,
    latencyResults, isTestingLatency, testingNodeTag, refreshingProfileId, activatingNodeTag, nodeGeoCache,
    diffPreview, isFetchingDiff, diffError, importPreview, isPreviewingImport, importPreviewError,
    setImportName, setImportContent, setImportError, pasteClipboard, pickFileAndImport, importConfig,
    selectNode, deleteProfile, testAllNodes, testNode, selectProfile, refreshSubscription,
    fetchSubscriptionDiff, clearDiffPreview, previewImportConfig, clearImportPreview,
    setProfileAutoUpdateInterval, createEmptyProfile, deleteNode,
  } = useProfileStore();
  const isConnected = useConnectionStore((state) => state.isConnected);
  const apiEnabled = useSettingsStore((state) => state.settings.apiEnabled);
  const openEditor = useNodeEditorStore((state) => state.openEditor);
  const stats = useStatsStore((state) => state.stats);
  const selectedProfile = profiles.find((profile) => profile.id === selectedProfileId) || null;
  const [showImport, setShowImport] = useState(false);
  const [diffProfileId, setDiffProfileId] = useState<string | null>(null);
  const dialogRef = useRef<HTMLDivElement>(null);
  const refreshTriggerRef = useRef<HTMLElement | null>(null);
  const trimmedContent = importContent.trim();
  const urlError = /^http/i.test(trimmedContent) && !/^https?:\/\/[^\s$.?#][^\s]*$/i.test(trimmedContent)
    ? 'Enter a valid subscription URL, such as https://example.com/config.'
    : null;

  const closeDiff = useCallback(() => {
    clearDiffPreview();
    setDiffProfileId(null);
    refreshTriggerRef.current?.focus();
  }, [clearDiffPreview]);

  const handleRequestRefresh = async (profileId: string, trigger: HTMLElement) => {
    refreshTriggerRef.current = trigger;
    setDiffProfileId(profileId);
    await fetchSubscriptionDiff(profileId);
  };

  const handleConfirmRefresh = async () => {
    if (diffProfileId && diffPreview?.previewToken && diffPreview.profileId === diffProfileId) {
      const targetId = diffProfileId;
      const token = diffPreview.previewToken;
      closeDiff();
      await refreshSubscription(targetId, token);
    }
  };

  useEffect(() => {
    if (!diffPreview) return;
    const dialog = dialogRef.current;
    const closeButton = dialog?.querySelector<HTMLElement>('[aria-label="Close diff preview"]');
    closeButton?.focus();
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        closeDiff();
      } else if (event.key === 'Tab' && dialog) {
        const buttons = Array.from(dialog.querySelectorAll<HTMLElement>('button, md-filled-button, md-outlined-button, md-text-button, md-icon-button'))
          .filter((button) => !button.hasAttribute('disabled'));
        if (buttons.length === 0) return;
        const first = buttons[0];
        const last = buttons[buttons.length - 1];
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first.focus();
        }
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [diffPreview, closeDiff]);

  useEffect(() => {
    if (selectedNodeTag) {
      document.getElementById(`node-card-${selectedNodeTag}`)?.scrollIntoView({
        behavior: window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ? 'instant' : 'smooth',
        block: 'nearest',
      });
    }
  }, [selectedNodeTag]);

  const handleFileImport = async () => {
    await pickFileAndImport();
    if (useProfileStore.getState().importSuccess) setShowImport(false);
  };

  const handleImport = async () => {
    await importConfig();
    if (useProfileStore.getState().importSuccess) setShowImport(false);
  };

  const handleContentChange = (value: string) => {
    setImportContent(value);
    if (importError) setImportError(null);
  };

  const handleDeleteProfile = async (profile: Profile) => {
    const confirmed = await ask(`Delete profile "${profile.name}"? This cannot be undone.`, {
      title: 'Delete Profile', kind: 'warning',
    });
    if (confirmed) await deleteProfile(profile.id);
  };

  const handleCreateEmpty = async () => {
    const name = importName.trim() || 'Manual Profile';
    await createEmptyProfile(name);
    setShowImport(false);
  };

  const handleDeleteNode = async (tag: string) => {
    const confirmed = await ask(`Delete node "${tag}"? This cannot be undone.`, {
      title: 'Delete Node', kind: 'warning',
    });
    if (confirmed) await deleteNode(tag);
  };

  return (
    <ViewShell title="Profiles" subtitle="Choose a configuration and select the server that carries your traffic"
      actions={<Button onClick={() => setShowImport((open) => !open)} aria-expanded={showImport} aria-controls="profile-import-panel">
        {showImport ? <Icon name="close" aria-hidden="true" /> : <Icon name="add" aria-hidden="true" />} {showImport ? 'Close import' : 'Add profile'}
      </Button>}>
      <div className={`profiles-layout ${showImport ? 'with-import' : ''}`}>
        {showImport && <section id="profile-import-panel" className="profiles-import material-panel" aria-labelledby="import-heading">
          <div className="material-section-heading">
            <div>
              <h2 id="import-heading">Add a profile</h2>
              <p>Paste a share link, subscription URL or configuration, or choose a file.</p>
            </div>
            <div className="flex gap-2">
              <Button variant="outline" onClick={() => void handleCreateEmpty()} disabled={isImporting} title="Create an empty manual profile">
                <Icon name="note_add" aria-hidden="true" /> Empty profile
              </Button>
              <Button variant="outline" onClick={() => void handleFileImport()} disabled={isImporting}>
                <Icon name="upload_file" aria-hidden="true" /> Choose file
              </Button>
            </div>
          </div>
          <form className="profiles-import-form" onSubmit={(event) => { event.preventDefault(); if (trimmedContent && !urlError && !isImporting) void handleImport(); }}>
            <FormInput label="Profile name (optional)" value={importName} onChange={setImportName} />
            <FormInput
              id="profile-content"
              label="Link or configuration"
              type="textarea"
              rows={3}
              value={importContent}
              onChange={handleContentChange}
              aria-invalid={Boolean(urlError)}
              aria-describedby={urlError ? 'profile-url-error' : undefined}
            />
            {urlError && <p id="profile-url-error" className="material-error" role="alert">{urlError}</p>}
            {importError && <p className="material-error" role="alert"><Icon name="warning" aria-hidden="true" /> {importError}</p>}
            {importSuccess && <p className="material-success" role="status"><Icon name="check" aria-hidden="true" /> Profile imported.</p>}
            {importPreview && (
              <div className="p-3 bg-muted/40 rounded-lg border border-border/60 flex flex-col gap-2 text-xs">
                <div className="flex items-center justify-between">
                  <span className="font-semibold text-foreground flex items-center gap-1.5">
                    <Icon name="check_circle" size={14} className="text-green-500" />
                    Found {importPreview.nodeCount} proxy node{importPreview.nodeCount === 1 ? '' : 's'}
                  </span>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    className="h-5 w-5 text-muted-foreground hover:text-foreground"
                    onClick={clearImportPreview}
                    title="Dismiss preview"
                    aria-label="Dismiss preview"
                  >
                    <Icon name="close" size={12} />
                  </Button>
                </div>
                <div className="flex flex-wrap gap-1.5 pt-0.5">
                  {Object.entries(importPreview.nodeTypes).map(([type, count]) => (
                    <Badge key={type} variant="secondary" className="text-[10px] font-mono">
                      {type}: {count}
                    </Badge>
                  ))}
                </div>
                {importPreview.sampleNodes.length > 0 && (
                  <div className="text-[11px] text-muted-foreground leading-normal">
                    Sample servers: <span className="font-medium text-foreground">{importPreview.sampleNodes.join(', ')}</span>
                    {importPreview.nodeCount > importPreview.sampleNodes.length && ` (and ${importPreview.nodeCount - importPreview.sampleNodes.length} more)`}
                  </div>
                )}
              </div>
            )}
            {importPreviewError && (
              <p className="material-error" role="alert">
                <Icon name="warning" aria-hidden="true" /> {importPreviewError}
              </p>
            )}
            <div className="profiles-import-actions">
              <Button variant="outline" onClick={pasteClipboard} disabled={isImporting}>
                <Icon name="content_paste" aria-hidden="true" /> Paste clipboard
              </Button>
              <Button
                type="button"
                variant="outline"
                disabled={isImporting || !trimmedContent || Boolean(urlError) || isPreviewingImport}
                onClick={() => void previewImportConfig(trimmedContent)}
                title="Inspect nodes before importing"
              >
                <Icon name={isPreviewingImport ? 'refresh' : 'visibility'} className={isPreviewingImport ? 'animate-spin' : undefined} aria-hidden="true" />
                {isPreviewingImport ? 'Previewing…' : 'Preview nodes'}
              </Button>
              <Button type="submit" disabled={isImporting || !trimmedContent || Boolean(urlError)} className="material-submit">
                <Icon name={isImporting ? "refresh" : "upload_file"} aria-hidden="true" className={isImporting ? "animate-spin" : undefined} />
                {isImporting ? 'Importing…' : 'Import profile'}
              </Button>
            </div>
          </form>
        </section>}

        <section className="profiles-list material-panel" aria-labelledby="profiles-heading">
          <div className="material-section-heading">
            <div>
              <h2 id="profiles-heading">Saved profiles <span className="material-count">{profiles.length}</span></h2>
              <p>Select a profile to view its servers.</p>
            </div>
          </div>
          <div className="profiles-items">
            {profiles.length ? profiles.map((profile) => {
              const isActive = profile.id === activeProfileId;
              const isSelected = profile.id === selectedProfileId;
              return (
                <div key={profile.id} className={`profile-item ${isSelected ? 'is-selected' : ''}`}>
                  <button type="button" className="profile-item-select" onClick={() => selectProfile(profile.id)} aria-current={isSelected ? 'true' : undefined}>
                    <span className="profile-item-icon">{profile.type === 'subscription' ? <Icon name="link" aria-hidden="true" /> : <Icon name="public" aria-hidden="true" />}</span>
                    <span className="profile-item-copy">
                      <strong title={profile.name}>{profile.name}</strong>
                      <span>{profile.type} · {profile.nodeCount} nodes · {new Date(profile.lastUpdated).toLocaleDateString()}{profile.autoUpdateInterval ? ` · Auto: ${profile.autoUpdateInterval >= 60 ? `${profile.autoUpdateInterval / 60}h` : `${profile.autoUpdateInterval}m`}` : ''}</span>
                    </span>
                    {isActive && <span className="material-status">{isConnected ? 'Connected' : 'Active'}</span>}
                  </button>
                  <div className="profile-item-actions">
                    {profile.type === 'subscription' && profile.subscriptionUrl && (
                      <Button variant="ghost" size="icon" aria-label={`Refresh ${profile.name}`} title={`Refresh ${profile.name}`} tabIndex={0}
                        disabled={refreshingProfileId === profile.id || (isFetchingDiff && diffProfileId === profile.id)}
                        onClick={(event) => void handleRequestRefresh(profile.id, event.currentTarget)}>
                        <Icon name="refresh" className={refreshingProfileId === profile.id || (isFetchingDiff && diffProfileId === profile.id) ? 'animate-spin' : ''} aria-hidden="true" />
                      </Button>
                    )}
                    <Button variant="ghost" size="icon" aria-label={`Delete ${profile.name}`} title={`Delete ${profile.name}`}
                      onClick={() => void handleDeleteProfile(profile)}>
                      <Icon name="delete" aria-hidden="true" />
                    </Button>
                  </div>
                </div>
              );
            }) : (
              <div className="material-empty">
                <div className="material-empty-icon" aria-hidden="true">
                  <Icon name="folder_open" />
                </div>
                <h3>No profiles yet</h3>
                <p>Add a subscription, paste a share link or choose a configuration file to get started.</p>
                <Button onClick={() => setShowImport(true)} className="material-action"><Icon name="add" aria-hidden="true" /> Add your first profile</Button>
              </div>
            )}
          </div>
        </section>

        <section className="profiles-servers material-panel" aria-labelledby="servers-heading">
          <div className="material-section-heading">
            <div>
              <h2 id="servers-heading">{selectedProfile ? selectedProfile.name : 'Servers'}</h2>
              <p>{selectedProfile ? `${selectedProfile.nodeCount} nodes · ${selectedProfile.type}${selectedProfile.id === activeProfileId ? ' · Active profile' : ''}` : 'Select a saved profile to see its servers.'}</p>
            </div>
            {selectedProfile && <div className="material-section-actions">
              {selectedProfile.type === 'subscription' && selectedProfile.subscriptionUrl && (
                <>
                  <select
                    className="material-select-compact"
                    value={selectedProfile.autoUpdateInterval || 0}
                    onChange={(e) => void setProfileAutoUpdateInterval(selectedProfile.id, Number(e.target.value))}
                    title="Auto-update interval"
                    aria-label="Auto-update interval"
                  >
                    <option value={0}>Auto-update: Off</option>
                    <option value={60}>Auto-update: 1h</option>
                    <option value={360}>Auto-update: 6h</option>
                    <option value={720}>Auto-update: 12h</option>
                    <option value={1440}>Auto-update: 24h</option>
                  </select>
                  <Button variant="outline" disabled={refreshingProfileId === selectedProfile.id || (isFetchingDiff && diffProfileId === selectedProfile.id)}
                    onClick={(event) => void handleRequestRefresh(selectedProfile.id, event.currentTarget)}>
                    <Icon name="refresh" className={refreshingProfileId === selectedProfile.id || (isFetchingDiff && diffProfileId === selectedProfile.id) ? 'animate-spin' : ''} aria-hidden="true" />
                    {isFetchingDiff && diffProfileId === selectedProfile.id ? 'Checking…' : 'Refresh'}
                  </Button>
                </>
              )}
              {selectedProfile.id === activeProfileId && (
                <Button variant="outline" onClick={() => openEditor(null)}>
                  <Icon name="add" aria-hidden="true" /> Add node
                </Button>
              )}
              <Button variant="outline" disabled={isTestingLatency || Boolean(testingNodeTag) || !nodes.length} onClick={testAllNodes}
                title={isConnected && apiEnabled && selectedProfile.id === activeProfileId ? 'Tests proxy URL through each node' : 'Tests direct TCP reachability of each node server'}>
                <Icon name="speed" aria-hidden="true" /> {isTestingLatency ? 'Testing…' : 'Test all nodes'}
              </Button>
            </div>}
          </div>
          {diffError && diffProfileId === selectedProfileId && <p role="alert" className="material-error">Could not preview subscription update: {diffError}</p>}
          {selectedProfile && nodes.length ? (
            <div className="server-items">
              {nodes.map((node: ProxyNode, index) => {
                const isSelected = selectedNodeTag === node.tag;
                const isActive = isConnected && selectedProfile.id === activeProfileId && isSelected;
                const latency = latencyResults[node.tag];
                const geo = nodeGeoCache[node.server];
                const country = geo && geo !== 'loading' ? geo.countryCode : getCountryCode(node.tag);
                const usage = stats[node.tag];
                return (
                  <div key={`${node.tag}-${index}`} id={`node-card-${node.tag}`} className={`server-item ${isSelected ? 'is-selected' : ''}`}>
                    <button type="button" className="server-item-select" onClick={() => selectNode(node)} aria-pressed={isSelected}>
                      <span className="server-item-icon" aria-hidden="true">{country && country.length === 2 ? country.toUpperCase() : <Icon name="public" />}</span>
                      <span className="server-item-copy">
                        <strong title={node.tag}>{node.tag}</strong>
                        <span>{node.type}{usage && (usage.totalUploadBytes > 0 || usage.totalDownloadBytes > 0) ? ` · ${formatBytes(usage.totalUploadBytes + usage.totalDownloadBytes)} used` : ''}</span>
                      </span>
                      {isActive && <span className="material-status">Connected</span>}
                      {activatingNodeTag === node.tag && <span className="material-status">Connecting…</span>}
                      {latency && <span className="server-latency" title={latency.error || `${latency.mode === 'tcp' ? 'Direct server TCP connect' : 'Proxy URL round trip'} latency`}>
                        {latency.mode === 'tcp' ? 'TCP' : 'Proxy URL'}: {latency.latencyMs !== null ? `${latency.latencyMs} ms` : 'Failed'}
                      </span>}
                    </button>
                    <div className="server-item-actions">
                      <Button variant="ghost" size="icon" aria-label={`Test direct TCP for ${node.tag}`} title={`Test direct TCP for ${node.tag}`}
                        disabled={isTestingLatency || Boolean(testingNodeTag)} onClick={() => void testNode(node, 'tcp')}>
                        <Icon name="network_ping" aria-hidden="true" />
                      </Button>
                      {isConnected && apiEnabled && selectedProfile.id === activeProfileId && (
                        <Button variant="ghost" size="icon" aria-label={`Test proxy URL for ${node.tag}`} title={`Test proxy URL for ${node.tag}`}
                          disabled={isTestingLatency || Boolean(testingNodeTag)} onClick={() => void testNode(node, 'proxy-url')}>
                          <Icon name="speed" aria-hidden="true" />
                        </Button>
                      )}
                      {selectedProfile.id === activeProfileId && (
                        <>
                          <Button variant="ghost" size="icon" aria-label={`Edit ${node.tag}`} title={`Edit ${node.tag}`} onClick={() => openEditor(node)}>
                            <Icon name="edit" aria-hidden="true" />
                          </Button>
                          <Button variant="ghost" size="icon" aria-label={`Delete ${node.tag}`} title={`Delete ${node.tag}`} onClick={() => void handleDeleteNode(node.tag)}>
                            <Icon name="delete" aria-hidden="true" />
                          </Button>
                        </>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          ) : (
            <div className="material-empty material-empty-compact">
              <div className="material-empty-icon" aria-hidden="true">
                <Icon name={selectedProfile ? "bolt" : "dns"} />
              </div>
              <h3>{selectedProfile ? 'No servers available' : 'No profile selected'}</h3>
              <p>{selectedProfile ? 'This profile has no selectable proxy nodes.' : 'Choose a profile to see its servers, or add a new one.'}</p>
            </div>
          )}
        </section>
      </div>

      {/* Subscription Diff Preview Modal */}
      {diffPreview && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
          <div className="fixed inset-0 bg-black/60 backdrop-blur-xs" onClick={closeDiff} aria-hidden="true" />
          <div
            ref={dialogRef}
            role="dialog"
            aria-modal="true"
            aria-labelledby="diff-dialog-title"
            className="relative w-full max-w-lg bg-card text-foreground rounded-2xl shadow-2xl border border-border overflow-hidden flex flex-col max-h-[85vh] animate-in fade-in-0 zoom-in-95 duration-200"
          >
            <div className="flex items-center justify-between p-4 border-b border-border bg-muted/20">
              <div>
                <h3 id="diff-dialog-title" className="text-sm font-bold text-foreground flex items-center gap-2">
                  <Icon name="compare_arrows" size={16} className="text-primary" aria-hidden="true" />
                  Subscription Update Preview
                </h3>
                <p className="text-[11px] text-muted-foreground mt-0.5">
                  Profile: <span className="font-semibold text-foreground">{diffPreview.profileName}</span>
                </p>
              </div>
              <Button variant="ghost" size="icon" className="h-7 w-7 text-muted-foreground hover:text-foreground" onClick={closeDiff} aria-label="Close diff preview" tabIndex={0}>
                <Icon name="close" size={14} />
              </Button>
            </div>

            <div className="p-4 flex flex-col gap-3.5 overflow-y-auto">
              {/* Node stats summary */}
              <div className="grid grid-cols-3 gap-2">
                <div className="bg-muted/40 p-2.5 rounded-lg border border-border/40 flex flex-col gap-0.5">
                  <span className="text-[10px] text-muted-foreground uppercase tracking-wider font-medium">Nodes</span>
                  <span className="text-xs font-semibold font-mono">
                    {diffPreview.oldNodeCount} → {diffPreview.newNodeCount}
                  </span>
                </div>
                <div className="bg-muted/40 p-2.5 rounded-lg border border-border/40 flex flex-col gap-0.5">
                  <span className="text-[10px] text-green-600 dark:text-green-400 uppercase tracking-wider font-medium">Added</span>
                  <span className="text-xs font-semibold text-green-600 dark:text-green-400 font-mono">
                    +{diffPreview.addedNodes.length}
                  </span>
                </div>
                <div className="bg-muted/40 p-2.5 rounded-lg border border-border/40 flex flex-col gap-0.5">
                  <span className="text-[10px] text-destructive uppercase tracking-wider font-medium">Removed</span>
                  <span className="text-xs font-semibold text-destructive font-mono">
                    -{diffPreview.removedNodes.length}
                  </span>
                </div>
              </div>

              {/* Selected node status warning */}
              {!diffPreview.isSelectedNodePreserved ? (
                <div className="bg-destructive/10 text-destructive border border-destructive/25 p-3 rounded-lg flex items-start gap-2.5 text-xs">
                  <Icon name="warning" size={18} className="shrink-0 mt-0.5" />
                  <div className="flex flex-col gap-1">
                    <span className="font-bold">Active server removed by provider!</span>
                    <p className="text-[11.5px] leading-relaxed text-foreground/90">
                      Your currently selected server <code className="font-mono bg-destructive/15 px-1 py-0.5 rounded text-destructive">{diffPreview.currentSelectedTag}</code> is no longer available in this subscription.
                      If applied, your active server will automatically fallback to <code className="font-mono bg-background px-1 py-0.5 rounded text-foreground border border-border">{diffPreview.fallbackSelectedTag || 'first available'}</code>.
                    </p>
                  </div>
                </div>
              ) : diffPreview.currentSelectedTag ? (
                <div className="bg-green-500/10 text-green-700 dark:text-green-400 border border-green-500/20 p-2.5 rounded-lg flex items-center gap-2 text-xs">
                  <Icon name="check_circle" size={16} className="shrink-0" />
                  <span>Active server <strong className="font-mono">{diffPreview.currentSelectedTag}</strong> will remain active.</span>
                </div>
              ) : null}

              {/* Added nodes list */}
              {diffPreview.addedNodes.length > 0 && (
                <div className="flex flex-col gap-1.5">
                  <span className="text-[10.5px] font-semibold text-muted-foreground uppercase tracking-wider">
                    Added Servers ({diffPreview.addedNodes.length})
                  </span>
                  <div className="flex flex-wrap gap-1 max-h-32 overflow-y-auto p-2 bg-muted/30 rounded-lg border border-border/40">
                    {diffPreview.addedNodes.map((tag) => (
                      <Badge key={tag} variant="outline" className="text-[10px] bg-green-500/10 text-green-700 dark:text-green-400 border-green-500/30">
                        + {tag}
                      </Badge>
                    ))}
                  </div>
                </div>
              )}

              {/* Removed nodes list */}
              {diffPreview.removedNodes.length > 0 && (
                <div className="flex flex-col gap-1.5">
                  <span className="text-[10.5px] font-semibold text-muted-foreground uppercase tracking-wider">
                    Removed Servers ({diffPreview.removedNodes.length})
                  </span>
                  <div className="flex flex-wrap gap-1 max-h-32 overflow-y-auto p-2 bg-muted/30 rounded-lg border border-border/40">
                    {diffPreview.removedNodes.map((tag) => (
                      <Badge key={tag} variant="outline" className="text-[10px] bg-destructive/10 text-destructive border-destructive/30">
                        - {tag}
                      </Badge>
                    ))}
                  </div>
                </div>
              )}

              {diffPreview.addedNodes.length === 0 && diffPreview.removedNodes.length === 0 && (
                <div className="p-3 bg-muted/30 rounded-lg border border-border/40 text-center text-xs text-muted-foreground">
                  No server tag changes detected. Node settings may still have changed.
                </div>
              )}
              <p className="text-[11px] text-muted-foreground">This preview expires after five minutes. Applying uses the previewed nodes; if local settings change, preview again.</p>
            </div>

            <div className="p-3 px-4 border-t border-border bg-muted/20 flex items-center justify-end gap-2">
              <Button variant="outline" size="sm" onClick={closeDiff}>
                Cancel
              </Button>
              <Button
                variant="default"
                size="sm"
                className="gap-1.5"
                onClick={() => void handleConfirmRefresh()}
                disabled={!diffPreview.previewToken}
              >
                <Icon name="refresh" size={14} />
                Apply Update
              </Button>
            </div>
          </div>
        </div>
      )}
    </ViewShell>
  );
}
