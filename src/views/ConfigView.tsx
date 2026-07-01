import { useEffect } from 'react';
import { Globe, RefreshCw, Check, ShieldAlert, Clipboard, FileUp, Edit3, Zap, CornerDownLeft, Trash2, ArrowRight, Link2, Activity } from 'lucide-react';
import { ViewShell } from '../components/ViewShell';
import { useProfileStore, getCountryCode } from '../stores/profileStore';
import { useConnectionStore } from '../stores/connectionStore';
import { useNodeEditorStore } from '../stores/nodeEditorStore';
import type { Profile } from '../utils/store';

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
  nodes: any[];
  selectedNodeTag: string | null;
  isConnected: boolean;
  nodeGeoCache: Record<string, string>;
  latencyResults: Record<string, { latencyMs: number | null; error: string | null }>;
}) {
  const activeNode = selectedNodeTag ? nodes.find((n: any) => n.tag === selectedNodeTag) : null;
  const activeServerCode = activeNode
    ? (nodeGeoCache[activeNode.server] && nodeGeoCache[activeNode.server] !== 'loading' && nodeGeoCache[activeNode.server] !== 'unknown'
        ? nodeGeoCache[activeNode.server]
        : getCountryCode(activeNode.tag))
    : null;
  const isActiveProfile = selectedProfile.id === activeProfileId;
  const activePing = activeNode && latencyResults[activeNode.tag] ? latencyResults[activeNode.tag] : null;

  return (
    <div style={{ padding: '16px 20px', background: 'var(--surface-sunken)', borderRadius: 'var(--r-md)', border: '1px solid var(--border-subtle)', marginBottom: '16px', flexShrink: 0 }}>
      {/* Top row: Profile name + badges */}
      <div style={{ marginBottom: activeNode ? '12px' : '0' }}>
        <h2 style={{ fontSize: '18px', fontWeight: 700, color: 'var(--text-high)', marginBottom: '6px' }}>
          {selectedProfile.name}
        </h2>
        <div style={{ display: 'flex', gap: '6px', flexWrap: 'wrap' }}>
          <span className="node-type-badge" style={{ fontSize: '10px', padding: '2px 7px', height: '18px', color: 'var(--accent-secondary)', borderColor: 'rgba(124,141,255,0.2)', background: 'rgba(124,141,255,0.06)' }}>
            <Zap size={9} style={{ marginRight: '3px' }} /> {selectedProfile.nodeCount} Nodes
          </span>
          {selectedProfile.type === 'subscription' && (selectedProfile as any).subscriptionUrl && (
            <span className="node-type-badge" style={{ fontSize: '10px', padding: '2px 7px', height: '18px', color: 'var(--accent-primary)', borderColor: 'var(--border-accent-dim)', background: 'var(--accent-primary-dim)' }}>
              <Link2 size={9} style={{ marginRight: '3px' }} /> Sub
            </span>
          )}
        </div>
      </div>

      {/* Active server details strip */}
      {activeNode && (
        <div style={{
          display: 'flex',
          alignItems: 'center',
          gap: '10px',
          padding: '10px 12px',
          background: isActiveProfile && isConnected ? 'rgba(34,197,94,0.06)' : 'var(--surface-overlay)',
          border: `1px solid ${isActiveProfile && isConnected ? 'rgba(34,197,94,0.15)' : 'var(--border-subtle)'}`,
          borderRadius: 'var(--r-md)',
        }}>
          {/* Country flag */}
          <span style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', width: '24px', height: '16px', flexShrink: 0 }}>
            {activeServerCode ? (
              <img
                src={`https://flagcdn.com/w40/${activeServerCode}.png`}
                alt={activeServerCode}
                style={{ width: '24px', height: '16px', objectFit: 'cover', borderRadius: '2px', boxShadow: '0 1px 3px rgba(0,0,0,0.25)' }}
                onError={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }}
              />
            ) : (
              <Globe size={14} style={{ color: 'var(--text-low)' }} />
            )}
          </span>

          {/* Server tag */}
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-high)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {activeNode.tag}
            </div>
            <span style={{ fontSize: '10px', color: 'var(--text-low)' }}>{activeNode.type}</span>
          </div>

          {/* Ping / Latency instead of LIVE badge */}
          {activePing && (
            <span style={{
              fontSize: '11px',
              fontWeight: 700,
              fontFamily: "'JetBrains Mono', monospace",
              fontVariantNumeric: 'tabular-nums',
              padding: '3px 8px',
              borderRadius: 'var(--r-pill)',
              flexShrink: 0,
              ...(activePing.latencyMs !== null
                ? activePing.latencyMs < 200
                  ? { color: 'var(--status-ok)', background: 'var(--status-ok-dim)' }
                  : activePing.latencyMs < 500
                    ? { color: 'var(--status-warn)', background: 'var(--status-warn-dim)' }
                    : { color: 'var(--status-err)', background: 'var(--status-err-dim)' }
                : { color: 'var(--text-low)', background: 'var(--surface-sunken)' }),
            }}>
              {activePing.latencyMs !== null ? `${activePing.latencyMs}ms` : 'timeout'}
            </span>
          )}
        </div>
      )}
    </div>
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
  nodeGeoCache,
  fetchNodeGeo,
  activatingNodeTag
}: {
  node: any;
  selectedNodeTag: string | null;
  selectNode: (node: any) => void;
  isConnected: boolean;
  selectedProfile: any;
  activeProfileId: string | null;
  openEditor: (node: any) => void;
  latencyResults: any;
  nodeGeoCache: Record<string, string>;
  fetchNodeGeo: (server: string, tag: string) => void;
  activatingNodeTag: string | null;
}) {
  useEffect(() => {
    const cachedCode = nodeGeoCache[node.server];
    if (!cachedCode && node.server) {
      fetchNodeGeo(node.server, node.tag);
    }
  }, [node.server, node.tag, nodeGeoCache, fetchNodeGeo]);

  return (
    <div
      className={`node-card ${selectedNodeTag === node.tag ? 'active' : ''}`}
      onClick={() => selectNode(node)}
      style={{ display: 'flex', flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', minWidth: 0 }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: '10px', overflow: 'hidden', flex: 1, minWidth: 0 }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', overflow: 'hidden', flex: 1, minWidth: 0 }}>
          <span className="node-name" title={node.tag} style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{node.tag}</span>
          <div style={{ display: 'flex', gap: '6px', alignItems: 'center', flexWrap: 'wrap' }}>
            <span className="node-type-badge" style={{ alignSelf: 'flex-start' }}>{node.type}</span>
            {isConnected && selectedProfile.id === activeProfileId && selectedNodeTag === node.tag ? (
              <span className="active-badge" style={{ fontSize: '9px', padding: '1px 6px' }}>ACTIVE</span>
            ) : (
              activatingNodeTag === node.tag && (
                <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                  <div className="blink-dot" />
                  <span style={{ fontSize: '9px', color: 'var(--text-low)', textTransform: 'uppercase', fontWeight: 600 }}>connecting</span>
                </div>
              )
            )}
          </div>
        </div>
      </div>
      <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flexShrink: 0, marginLeft: '12px' }}>
        {selectedProfile.id === activeProfileId && (
          <button
            className="btn-icon-only"
            style={{ width: '28px', height: '28px', border: 'none', background: 'var(--surface-sunken)', color: 'var(--text-low)', borderRadius: '6px' }}
            onClick={(e) => { e.stopPropagation(); openEditor(node); }}
            title="Edit node configuration"
          >
            <Edit3 size={13} />
          </button>
        )}
        {latencyResults[node.tag] && (
          <span
            style={{
              fontSize: '10px',
              fontWeight: 600,
              padding: '2px 6px',
              borderRadius: '4px',
              fontVariantNumeric: 'tabular-nums',
              ...(latencyResults[node.tag].latencyMs !== null
                ? latencyResults[node.tag].latencyMs! < 200
                  ? { color: 'var(--status-ok)', background: 'var(--status-ok-dim)' }
                  : latencyResults[node.tag].latencyMs! < 500
                    ? { color: 'var(--status-warn)', background: 'var(--status-warn-dim)' }
                    : { color: 'var(--status-err)', background: 'var(--status-err-dim)' }
                : { color: 'var(--text-low)', background: 'var(--surface-sunken)' }),
            }}
            title={latencyResults[node.tag].error || undefined}
          >
            {latencyResults[node.tag].latencyMs !== null
              ? `${latencyResults[node.tag].latencyMs}ms`
              : 'timeout'}
          </span>
        )}
      </div>
    </div>
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
    fetchNodeGeo,
    activatingNodeTag,
  } = useProfileStore();

  const { isConnected } = useConnectionStore();
  const { openEditor } = useNodeEditorStore();

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

  const getProfileTypeColor = (type: Profile['type']) => {
    switch (type) {
      case 'subscription': return { color: 'var(--accent-primary)', bg: 'var(--accent-primary-dim)', border: 'var(--border-accent-dim)' };
      case 'file': return { color: 'var(--accent-secondary)', bg: 'var(--accent-secondary-dim)', border: 'rgba(124,141,255,0.15)' };
      case 'manual': return { color: 'var(--status-warn)', bg: 'var(--status-warn-dim)', border: 'rgba(245,158,11,0.15)' };
    }
  };

  return (
    <ViewShell title="Profiles" subtitle="Manage proxy configurations and server nodes">
      <div className="grid-2" style={{ height: 'calc(100vh - 100px)' }}>
        {/* Left panel: Profile list + Import */}
        <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>

          {/* Import Form */}
          <div style={{ flexShrink: 0, padding: '16px', marginBottom: '16px', background: 'var(--surface-sunken)', borderRadius: 'var(--r-md)', border: '1px solid var(--border-subtle)' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '12px' }}>
              <h3 style={{ fontSize: '13px', fontWeight: 600, color: 'var(--text-med)', display: 'flex', alignItems: 'center', gap: '6px' }}>
                <FileUp size={14} /> Add Profile
              </h3>
              <button
                type="button"
                className="btn secondary"
                onClick={pickFileAndImport}
                disabled={isImporting}
                style={{ padding: '0 8px', height: '24px', fontSize: '11px', display: 'flex', alignItems: 'center', gap: '4px' }}
                title="Choose config file to import"
              >
                <FileUp size={12} /> Choose File
              </button>
            </div>
            <form onSubmit={handleImportSubmit} style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              <input
                className="text-input"
                style={{ width: '100%', fontSize: '12px', height: '32px' }}
                value={importName}
                onChange={(e) => setImportName(e.target.value)}
                placeholder="Profile Name (Optional)"
              />
              <div style={{ display: 'flex', gap: '8px' }}>
                <input
                  className="text-input"
                  style={{ flex: 1, fontSize: '12px' }}
                  value={importContent}
                  onChange={(e) => setImportContent(e.target.value)}
                  placeholder="Paste vless://, ss://, subscription URL…"
                />
                <button
                  type="button"
                  className="btn secondary"
                  onClick={pasteClipboard}
                  title="Paste from Clipboard"
                  style={{ padding: '0 12px', height: '32px' }}
                >
                  <Clipboard size={14} />
                </button>
                <button
                  type="submit"
                  className="btn primary"
                  disabled={isImporting || !importContent.trim()}
                  style={{ padding: '0 12px', height: '32px' }}
                  title="Import"
                >
                  {isImporting ? <RefreshCw size={14} className="spin" /> : <CornerDownLeft size={16} />}
                </button>
              </div>
              {importError && (
                <div style={{ background: 'var(--status-err-dim)', color: 'var(--status-err)', fontSize: '11px', padding: '6px 10px', borderRadius: '4px', display: 'flex', alignItems: 'center', gap: '6px' }}>
                  <ShieldAlert size={12} /> {importError}
                </div>
              )}
              {importSuccess && (
                <div style={{ background: 'var(--status-ok-dim)', color: 'var(--status-ok)', fontSize: '11px', padding: '6px 10px', borderRadius: '4px', display: 'flex', alignItems: 'center', gap: '6px' }}>
                  <Check size={12} /> Profile imported successfully!
                </div>
              )}
            </form>
          </div>

          {/* Profile List */}
          <div className="flex-row-between" style={{ marginBottom: '12px', flexShrink: 0 }}>
            <h3 style={{ fontSize: '14px', fontWeight: 600 }}>
              Profiles
              {profiles.length > 0 && <span style={{ color: 'var(--text-low)', fontWeight: 400, marginLeft: '6px', fontSize: '12px' }}>({profiles.length})</span>}
            </h3>
          </div>
          <div style={{ flex: 1, overflowY: 'auto', paddingRight: '4px' }}>
            {profiles.length > 0 ? (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                {profiles.map((profile) => {
                  const isActive = profile.id === activeProfileId;
                  const isSelected = profile.id === selectedProfileId;
                  const typeStyle = getProfileTypeColor(profile.type);
                  return (
                    <div
                      key={profile.id}
                      className={`profile-item ${isSelected ? 'active' : ''}`}
                      onClick={() => !isSelected && selectProfile(profile.id)}
                      style={{ cursor: isSelected ? 'default' : 'pointer', position: 'relative' }}
                    >
                      <div className="profile-item-left">
                        <div className="profile-icon">
                          {profile.type === 'subscription' ? <Link2 size={18} /> : <Globe size={18} />}
                        </div>
                        <div className="profile-details">
                          <span className="profile-name" title={profile.name}>
                            {profile.name.length > 15 ? `${profile.name.slice(0, 15)}...` : profile.name}
                          </span>
                          <div className="profile-meta">
                            <span style={{
                              fontSize: '9px',
                              fontWeight: 600,
                              padding: '1px 5px',
                              borderRadius: '3px',
                              color: typeStyle.color,
                              background: typeStyle.bg,
                              border: `1px solid ${typeStyle.border}`,
                              letterSpacing: '0.5px',
                            }}>
                              {getProfileTypeLabel(profile.type)}
                            </span>
                            <span>{profile.nodeCount} nodes</span>
                            <span>•</span>
                            <span>{new Date(profile.lastUpdated).toLocaleDateString()}</span>
                          </div>
                        </div>
                      </div>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                        {isActive && (
                          <div title={isConnected ? "Active & Connected" : "Active"} style={{
                            width: '8px', height: '8px', borderRadius: '50%',
                            background: 'var(--status-ok)',
                            boxShadow: isConnected ? '0 0 8px rgba(46, 213, 115, 0.6)' : 'none',
                          }} />
                        )}
                        {!isSelected && !isActive && (
                          <ArrowRight size={14} style={{ color: 'var(--text-low)', opacity: 0.5 }} />
                        )}
                        <button
                          className="btn-icon-only"
                          style={{ width: '26px', height: '26px', border: 'none', background: 'transparent', color: 'var(--text-low)', flexShrink: 0, borderRadius: '4px' }}
                          onClick={(e) => handleDeleteProfile(e, profile)}
                          title="Delete profile"
                        >
                          <Trash2 size={12} />
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            ) : (
              <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', color: 'var(--text-low)', height: '100%', minHeight: '120px' }}>
                <Globe size={32} style={{ strokeWidth: 1.5, marginBottom: '12px', opacity: 0.4 }} />
                <p style={{ fontSize: '13px', fontWeight: 500 }}>No Profiles Yet</p>
                <p style={{ fontSize: '11px', marginTop: '4px', textAlign: 'center', maxWidth: '220px' }}>Add a profile above using a subscription link, config file, or pasting node URIs.</p>
              </div>
            )}
          </div>
        </div>

        {/* Right panel: Node list for active profile */}
        <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
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

              <div className="flex-row-between" style={{ marginBottom: '12px', flexShrink: 0, paddingLeft: '4px' }}>
                <h3 style={{ fontSize: '14px', fontWeight: 600 }}>Servers</h3>
                <button
                  className="btn secondary"
                  onClick={testAllNodes}
                  disabled={isTestingLatency || nodes.length === 0}
                  style={{ padding: '0 10px', height: '28px', fontSize: '11px', display: 'flex', alignItems: 'center', gap: '5px' }}
                  title="Test TCP latency for all nodes"
                >
                  {isTestingLatency ? <RefreshCw size={12} className="spin" /> : <Activity size={12} />}
                  {isTestingLatency ? 'Testing…' : 'Test All'}
                </button>
              </div>

              <div style={{ flex: 1, overflowY: 'auto', paddingRight: '4px' }}>
                {nodes.length > 0 ? (
                  <div className="node-grid">
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
                        nodeGeoCache={nodeGeoCache}
                        fetchNodeGeo={fetchNodeGeo}
                        activatingNodeTag={activatingNodeTag}
                      />
                    ))}
                  </div>
                ) : (
                  <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', height: '100px', color: 'var(--text-low)' }}>
                    <p style={{ fontSize: '13px' }}>No customizable servers found.</p>
                  </div>
                )}
              </div>
            </>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', flex: 1, color: 'var(--text-low)' }}>
              <div style={{ width: '80px', height: '80px', borderRadius: '50%', background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(255,255,255,0.05)', display: 'flex', alignItems: 'center', justifyContent: 'center', marginBottom: '24px', boxShadow: '0 0 30px rgba(74,158,255,0.05)' }}>
                <Globe size={36} style={{ strokeWidth: 1.5, color: 'var(--accent-primary)', opacity: 0.8 }} />
              </div>
              <p style={{ fontSize: '16px', fontWeight: 600, color: 'var(--text-high)' }}>No Profile Selected</p>
              <p style={{ fontSize: '13px', marginTop: '8px', maxWidth: '280px', textAlign: 'center', lineHeight: '1.5' }}>
                Add a profile on the left to see its available servers here.
              </p>
            </div>
          )}
        </div>
      </div>
    </ViewShell>
  );
}
