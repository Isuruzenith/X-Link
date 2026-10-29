import { useEffect, useState } from 'react';
import { Icon } from '@/components/ui/icon';
import { ask } from '@tauri-apps/plugin-dialog';
import { ViewShell } from '../components/ViewShell';
import { Button } from '@/components/ui/button';
import { FormInput } from '@/components/form';
import { useProfileStore, getCountryCode } from '../stores/profileStore';
import { useConnectionStore } from '../stores/connectionStore';
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
    latencyResults, isTestingLatency, refreshingProfileId, activatingNodeTag, nodeGeoCache,
    setImportName, setImportContent, setImportError, pasteClipboard, pickFileAndImport, importConfig,
    selectNode, deleteProfile, testAllNodes, selectProfile, refreshSubscription,
    setProfileAutoUpdateInterval,
  } = useProfileStore();
  const isConnected = useConnectionStore((state) => state.isConnected);
  const openEditor = useNodeEditorStore((state) => state.openEditor);
  const stats = useStatsStore((state) => state.stats);
  const selectedProfile = profiles.find((profile) => profile.id === selectedProfileId) || null;
  const [showImport, setShowImport] = useState(false);
  const trimmedContent = importContent.trim();
  const urlError = /^http/i.test(trimmedContent) && !/^https?:\/\/[^\s$.?#][^\s]*$/i.test(trimmedContent)
    ? 'Enter a valid subscription URL, such as https://example.com/config.'
    : null;

  useEffect(() => {
    if (selectedNodeTag) {
      document.getElementById(`node-card-${selectedNodeTag}`)?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
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
            <Button variant="outline" onClick={() => void handleFileImport()} disabled={isImporting}>
              <Icon name="upload_file" aria-hidden="true" /> Choose file
            </Button>
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
            <div className="profiles-import-actions">
              <Button variant="outline" onClick={pasteClipboard} disabled={isImporting}>
                <Icon name="content_paste" aria-hidden="true" /> Paste clipboard
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
                      <Button variant="ghost" size="icon" aria-label={`Refresh ${profile.name}`} title={`Refresh ${profile.name}`}
                        disabled={refreshingProfileId === profile.id} onClick={() => refreshSubscription(profile.id)}>
                        <Icon name="refresh" className={refreshingProfileId === profile.id ? 'animate-spin' : ''} aria-hidden="true" />
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
                <Icon name="public" aria-hidden="true" />
                <h3>No profiles yet</h3>
                <p>Add a subscription, paste a share link or choose a configuration file to get started.</p>
                <Button onClick={() => setShowImport(true)}><Icon name="add" aria-hidden="true" /> Add your first profile</Button>
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
                  <Button variant="outline" disabled={refreshingProfileId === selectedProfile.id}
                    onClick={() => refreshSubscription(selectedProfile.id)}>
                    <Icon name="refresh" className={refreshingProfileId === selectedProfile.id ? 'animate-spin' : ''} aria-hidden="true" /> Refresh
                  </Button>
                </>
              )}
              {selectedProfile.id === activeProfileId && (
                <Button variant="outline" onClick={() => openEditor(null)}>
                  <Icon name="add" aria-hidden="true" /> Add node
                </Button>
              )}
              <Button variant="outline" disabled={isTestingLatency || !nodes.length} onClick={testAllNodes}>
                <Icon name="speed" aria-hidden="true" /> {isTestingLatency ? 'Testing…' : 'Test latency'}
              </Button>
            </div>}
          </div>
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
                      {latency && <span className="server-latency" title={latency.error || undefined}>{latency.latencyMs !== null ? `${latency.latencyMs} ms` : 'Timeout'}</span>}
                    </button>
                    {selectedProfile.id === activeProfileId && (
                      <Button variant="ghost" size="icon" aria-label={`Edit ${node.tag}`} title={`Edit ${node.tag}`} onClick={() => openEditor(node)}>
                        <Icon name="edit" aria-hidden="true" />
                      </Button>
                    )}
                  </div>
                );
              })}
            </div>
          ) : (
            <div className="material-empty">
              {selectedProfile ? <Icon name="bolt" aria-hidden="true" /> : <Icon name="public" aria-hidden="true" />}
              <h3>{selectedProfile ? 'No servers available' : 'No profile selected'}</h3>
              <p>{selectedProfile ? 'This profile has no selectable proxy nodes.' : 'Choose a profile to see its servers, or add a new one.'}</p>
            </div>
          )}
        </section>
      </div>
    </ViewShell>
  );
}
