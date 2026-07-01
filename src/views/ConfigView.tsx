import { Globe, RefreshCw, Check, ShieldAlert, Clipboard, FileUp, Edit3, Server, Zap, CornerDownLeft, Trash2, ArrowRight, Link2, Activity } from 'lucide-react';
import { ViewShell } from '../components/ViewShell';
import { useProfileStore } from '../stores/profileStore';
import { useConnectionStore } from '../stores/connectionStore';
import { useNodeEditorStore } from '../stores/nodeEditorStore';
import type { Profile } from '../utils/store';

export function ConfigView() {
  const {
    profiles,
    activeProfileId,
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
    switchProfile,
    deleteProfile,
    testAllNodes,
    activeProfile: getActiveProfile,
  } = useProfileStore();

  const { isConnected } = useConnectionStore();
  const { openEditor } = useNodeEditorStore();

  const activeProfile = getActiveProfile();

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
            <h3 style={{ fontSize: '13px', fontWeight: 600, color: 'var(--text-med)', marginBottom: '12px', display: 'flex', alignItems: 'center', gap: '6px' }}>
              <FileUp size={14} /> Add Profile
            </h3>
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
              <button
                type="button"
                className="btn secondary"
                onClick={pickFileAndImport}
                disabled={isImporting}
                style={{ width: '100%', height: '32px', fontSize: '12px', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '6px' }}
              >
                <FileUp size={13} /> Choose File…
              </button>
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
                  const typeStyle = getProfileTypeColor(profile.type);
                  return (
                    <div
                      key={profile.id}
                      className={`profile-item ${isActive ? 'active' : ''}`}
                      onClick={() => !isActive && switchProfile(profile.id)}
                      style={{ cursor: isActive ? 'default' : 'pointer', position: 'relative' }}
                    >
                      <div className="profile-item-left">
                        <div className="profile-icon">
                          {profile.type === 'subscription' ? <Link2 size={18} /> : <Globe size={18} />}
                        </div>
                        <div className="profile-details">
                          <span className="profile-name">{profile.name}</span>
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
                        {isActive && isConnected && (
                          <div title="Connected" style={{
                            width: '8px', height: '8px', borderRadius: '50%',
                            background: 'var(--status-ok)', boxShadow: '0 0 8px rgba(46, 213, 115, 0.6)',
                          }} />
                        )}
                        {!isActive && (
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
          {activeProfile ? (
            <>
              <div style={{ padding: '20px', background: 'var(--surface-sunken)', borderRadius: 'var(--r-md)', border: '1px solid var(--border-subtle)', marginBottom: '16px', flexShrink: 0 }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                  <div>
                    <h2 style={{ fontSize: '20px', fontWeight: 700, color: 'var(--text-high)', marginBottom: '8px' }}>
                      {activeProfile.name}
                    </h2>
                    <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
                      <span className="node-type-badge" style={{ fontSize: '11px', padding: '2px 8px', height: '20px', color: 'var(--accent-secondary)', borderColor: 'rgba(124,141,255,0.2)', background: 'rgba(124,141,255,0.06)' }}>
                        <Zap size={10} style={{ marginRight: '4px' }} /> {activeProfile.nodeCount} Nodes
                      </span>
                      {activeProfile.type === 'subscription' && activeProfile.subscriptionUrl && (
                        <span className="node-type-badge" style={{ fontSize: '11px', padding: '2px 8px', height: '20px', color: 'var(--accent-primary)', borderColor: 'var(--border-accent-dim)', background: 'var(--accent-primary-dim)' }}>
                          <Link2 size={10} style={{ marginRight: '4px' }} /> Subscription
                        </span>
                      )}
                    </div>
                  </div>
                </div>
              </div>

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
                      <div
                        key={i}
                        className={`node-card ${selectedNodeTag === node.tag ? 'active' : ''}`}
                        onClick={() => selectNode(node)}
                        style={{ display: 'flex', flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center' }}
                      >
                        <div style={{ display: 'flex', alignItems: 'center', gap: '10px', overflow: 'hidden' }}>
                          <Server size={14} style={{ color: selectedNodeTag === node.tag ? 'var(--accent-primary)' : 'var(--text-low)' }} />
                          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', overflow: 'hidden' }}>
                            <span className="node-name" title={node.tag}>{node.tag}</span>
                            <div style={{ display: 'flex', gap: '6px', alignItems: 'center' }}>
                              <span className="node-type-badge" style={{ alignSelf: 'flex-start' }}>{node.type}</span>
                              {isConnected && selectedNodeTag === node.tag && (
                                <span className="active-badge" style={{ fontSize: '9px', padding: '1px 6px' }}>ACTIVE</span>
                              )}
                            </div>
                          </div>
                        </div>
                        <button
                          className="btn-icon-only"
                          style={{ width: '28px', height: '28px', border: 'none', background: 'var(--surface-sunken)', color: 'var(--text-low)', flexShrink: 0, borderRadius: '6px' }}
                          onClick={(e) => { e.stopPropagation(); openEditor(node); }}
                          title="Edit node configuration"
                        >
                          <Edit3 size={13} />
                        </button>
                        {latencyResults[node.tag] && (
                          <span
                            style={{
                              fontSize: '10px',
                              fontWeight: 600,
                              padding: '2px 6px',
                              borderRadius: '4px',
                              flexShrink: 0,
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
                <Server size={36} style={{ strokeWidth: 1.5, color: 'var(--accent-primary)', opacity: 0.8 }} />
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
