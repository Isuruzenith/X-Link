import React from 'react';
import { Globe, Trash2, Plus, RefreshCw, Check, ShieldAlert, Clipboard, Edit3, Network, Server, Zap, BoxSelect, Inbox, CornerDownLeft } from 'lucide-react';
import { ViewShell } from '../components/ViewShell';
import type { Profile } from '../utils/store';

interface ProfilesViewProps {
  profiles: Profile[];
  selectedProfileId: string | null;
  activeProfileId: string | null;
  swappingProfileId: string | null;
  isConnected: boolean;
  importName: string;
  importContent: string;
  importError: string | null;
  importSuccess: boolean;
  isImporting: boolean;
  profileOutbounds: any[];
  selectedOutboundTag: string | null;
  onSelectProfile: (id: string) => void;
  onDeleteProfile: (id: string, e: React.MouseEvent) => void;
  onSetImportName: (v: string) => void;
  onSetImportContent: (v: string) => void;
  onPasteClipboard: () => void;
  onImportProfile: (e: React.FormEvent) => void;
  onSelectOutbound: (node: any) => void;
  onOpenEditor: (node: any) => void;
}

export function ProfilesView({
  profiles, selectedProfileId, activeProfileId, swappingProfileId, isConnected,
  importName, importContent, importError, importSuccess, isImporting,
  profileOutbounds, selectedOutboundTag,
  onSelectProfile, onDeleteProfile, onSetImportName, onSetImportContent,
  onPasteClipboard, onImportProfile, onSelectOutbound, onOpenEditor,
}: ProfilesViewProps) {
  return (
    <ViewShell title="Profiles" subtitle="Import and manage proxy subscription profiles">
      <div className="grid-2" style={{ height: 'calc(100vh - 100px)' }}>
        {/* Profile list and Import */}
        <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
          
          {/* Quick Import Form (Top) */}
          <div style={{ flexShrink: 0, padding: '16px', marginBottom: '16px', background: 'var(--surface-sunken)', borderRadius: 'var(--r-md)', border: '1px solid var(--border-subtle)' }}>
            <h3 style={{ fontSize: '13px', fontWeight: 600, color: 'var(--text-med)', marginBottom: '12px', display: 'flex', alignItems: 'center', gap: '6px' }}>
              <Plus size={14} /> Quick Add Subscription
            </h3>
            <form onSubmit={onImportProfile} style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              <input className="text-input" style={{ width: '100%', fontSize: '12px', height: '32px' }} 
                value={importName} onChange={(e) => onSetImportName(e.target.value)}
                placeholder="Profile Name (Optional)" />
              <div style={{ display: 'flex', gap: '8px' }}>
                <input className="text-input" style={{ flex: 1, fontSize: '12px' }} 
                  value={importContent} onChange={(e) => onSetImportContent(e.target.value)}
                  placeholder="Paste config URL or text..." />
                <button type="button" className="btn secondary" onClick={onPasteClipboard} title="Paste from Clipboard" style={{ padding: '0 12px', height: '32px' }}>
                  <Clipboard size={14} />
                </button>
                <button type="submit" className="btn primary" disabled={isImporting || !importContent.trim()} 
                  style={{ padding: '0 12px', height: '32px' }} title="Import">
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
                  <Check size={12} /> Imported successfully!
                </div>
              )}
            </form>
          </div>

          <div className="flex-row-between" style={{ marginBottom: '12px', flexShrink: 0 }}>
            <h3 style={{ fontSize: '14px', fontWeight: 600 }}>Subscriptions</h3>
          </div>
          <div style={{ flex: 1, overflowY: 'auto', paddingRight: '4px' }}>
            {profiles.length > 0 ? profiles.map((p) => (
              <div key={p.id} className={`profile-item ${selectedProfileId === p.id ? 'active' : ''}`}
                onClick={() => onSelectProfile(p.id)}>
                <div className="profile-item-left">
                  <div className="profile-icon"><Globe size={18} /></div>
                  <div className="profile-details">
                    <span className="profile-name">{p.name}</span>
                    <div className="profile-meta">
                      <span style={{ textTransform: 'capitalize' }}>{p.type}</span>
                      <span>•</span><span>{p.nodeCount} nodes</span>
                    </div>
                  </div>
                </div>
                <div className="profile-actions" style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
                  {swappingProfileId === p.id && (
                    <RefreshCw size={14} className="spin" style={{ color: 'var(--text-med)' }} title="Connecting..." />
                  )}
                  {activeProfileId === p.id && isConnected && swappingProfileId !== p.id && (
                    <div title="Active Profile" style={{
                      width: '8px',
                      height: '8px',
                      borderRadius: '50%',
                      background: 'var(--status-ok)',
                      boxShadow: '0 0 8px rgba(46, 213, 115, 0.6)'
                    }} />
                  )}
                  <button className="btn-icon-only danger" onClick={(e) => onDeleteProfile(p.id, e)} title="Delete">
                    <Trash2 size={14} />
                  </button>
                </div>
              </div>
            )) : (
              <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', color: 'var(--text-low)', height: '100%', minHeight: '120px' }}>
                <Inbox size={32} style={{ strokeWidth: 1.5, marginBottom: '12px', opacity: 0.6 }} />
                <p style={{ fontSize: '13px', fontWeight: 500 }}>No Subscriptions</p>
                <p style={{ fontSize: '11px', marginTop: '4px', textAlign: 'center', maxWidth: '200px' }}>Use the Quick Add form above to import your first configuration.</p>
              </div>
            )}
          </div>
        </div>

        {/* Detail panel */}
        <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
          {selectedProfileId ? (
            <>
              {/* Hero Header */}
              <div style={{ padding: '20px', background: 'var(--surface-sunken)', borderRadius: 'var(--r-md)', border: '1px solid var(--border-subtle)', marginBottom: '16px', flexShrink: 0 }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                  <div>
                    <h2 style={{ fontSize: '20px', fontWeight: 700, color: 'var(--text-high)', marginBottom: '8px' }}>
                      {profiles.find(p => p.id === selectedProfileId)?.name}
                    </h2>
                    <div style={{ display: 'flex', gap: '8px' }}>
                      <span className="node-type-badge" style={{ fontSize: '11px', padding: '2px 8px', height: '20px' }}>
                        Type: {profiles.find(p => p.id === selectedProfileId)?.type.toUpperCase()}
                      </span>
                      <span className="node-type-badge" style={{ fontSize: '11px', padding: '2px 8px', height: '20px', color: 'var(--accent-secondary)', borderColor: 'rgba(124,141,255,0.2)', background: 'rgba(124,141,255,0.06)' }}>
                        <Zap size={10} style={{ marginRight: '4px' }} /> {profiles.find(p => p.id === selectedProfileId)?.nodeCount} Nodes
                      </span>
                    </div>
                  </div>
                  <Network size={28} style={{ color: 'var(--accent-primary)', opacity: 0.8 }} />
                </div>
              </div>

              <div className="flex-row-between" style={{ marginBottom: '12px', flexShrink: 0, paddingLeft: '4px' }}>
                <h3 style={{ fontSize: '14px', fontWeight: 600 }}>Servers</h3>
              </div>
              
              <div style={{ flex: 1, overflowY: 'auto', paddingRight: '4px' }}>
                {profileOutbounds.length > 0 ? (
                  <div className="node-grid">
                    {profileOutbounds.map((node, i) => (
                      <div key={i} className={`node-card ${selectedOutboundTag === node.tag ? 'active' : ''}`} onClick={() => onSelectOutbound(node)} style={{ display: 'flex', flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center' }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '10px', overflow: 'hidden' }}>
                          <Server size={14} style={{ color: selectedOutboundTag === node.tag ? 'var(--accent-primary)' : 'var(--text-low)' }} />
                          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', overflow: 'hidden' }}>
                            <span className="node-name" title={node.tag}>{node.tag}</span>
                            <div style={{ display: 'flex', gap: '6px', alignItems: 'center' }}>
                              <span className="node-type-badge" style={{ alignSelf: 'flex-start' }}>{node.type}</span>
                              {activeProfileId === selectedProfileId && isConnected && selectedOutboundTag === node.tag && (
                                <span className="active-badge" style={{ fontSize: '9px', padding: '1px 6px' }}>ACTIVE</span>
                              )}
                            </div>
                          </div>
                        </div>
                        <button className="btn-icon-only" style={{ width: '28px', height: '28px', border: 'none', background: 'var(--surface-sunken)', color: 'var(--text-low)', flexShrink: 0, borderRadius: '6px' }}
                          onClick={(e) => { e.stopPropagation(); onOpenEditor(node); }} title="Edit node configuration">
                          <Edit3 size={13} />
                        </button>
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
                <BoxSelect size={36} style={{ strokeWidth: 1.5, color: 'var(--accent-primary)', opacity: 0.8 }} />
              </div>
              <p style={{ fontSize: '16px', fontWeight: 600, color: 'var(--text-high)' }}>Select a Subscription</p>
              <p style={{ fontSize: '13px', marginTop: '8px', maxWidth: '280px', textAlign: 'center', lineHeight: '1.5' }}>
                Choose a subscription from the left panel to explore and manage its available servers.
              </p>
            </div>
          )}
        </div>
      </div>
    </ViewShell>
  );
}
