import React from 'react';
import { Globe, Trash2, Plus, RefreshCw, Check, ShieldAlert, Clipboard, Edit3 } from 'lucide-react';
import { ViewShell } from '../components/ViewShell';
import type { Profile } from '../utils/store';

interface ProfilesViewProps {
  profiles: Profile[];
  selectedProfileId: string | null;
  activeProfileId: string | null;
  isConnected: boolean;
  importName: string;
  importContent: string;
  importError: string | null;
  importSuccess: boolean;
  isImporting: boolean;
  isImporting: boolean;
  profileOutbounds: any[];
  selectedOutboundTag: string | null;
  onSelectProfile: (id: string) => void;
  onDeleteProfile: (id: string, e: React.MouseEvent) => void;
  onClearSelection: () => void;
  onSetImportName: (v: string) => void;
  onSetImportContent: (v: string) => void;
  onPasteClipboard: () => void;
  onImportProfile: (e: React.FormEvent) => void;
  onSelectOutbound: (node: any) => void;
  onOpenEditor: (node: any) => void;
}

export function ProfilesView({
  profiles, selectedProfileId, activeProfileId, isConnected,
  importName, importContent, importError, importSuccess, isImporting,
  profileOutbounds, selectedOutboundTag,
  onSelectProfile, onDeleteProfile, onClearSelection, onSetImportName, onSetImportContent,
  onPasteClipboard, onImportProfile, onSelectOutbound, onOpenEditor,
}: ProfilesViewProps) {
  return (
    <ViewShell title="Profiles" subtitle="Import and manage proxy subscription profiles">
      <div className="grid-2" style={{ height: 'calc(100vh - 100px)' }}>
        {/* Profile list and Import */}
        <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
          
          {/* Quick Import Form (Top) */}
          <div style={{ flexShrink: 0, paddingBottom: '16px', borderBottom: '1px solid rgba(255,255,255,0.05)', marginBottom: '16px' }}>
            <form onSubmit={onImportProfile} style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              <input className="text-input" style={{ width: '100%', fontSize: '12px', height: '32px' }} 
                value={importName} onChange={(e) => onSetImportName(e.target.value)}
                placeholder="Profile Name (Required)" />
              <div style={{ display: 'flex', gap: '8px' }}>
                <button type="button" className="btn secondary" onClick={onPasteClipboard} title="Paste from Clipboard" style={{ padding: '0 12px', height: '32px' }}>
                  <Clipboard size={14} />
                </button>
                <input className="text-input" style={{ flex: 1, fontSize: '12px' }} 
                  value={importContent} onChange={(e) => onSetImportContent(e.target.value)}
                  placeholder="Paste config URL or text..." />
                <button type="submit" className="btn primary" disabled={isImporting || !importContent.trim()} 
                  style={{ padding: '0 12px', height: '32px' }} title="Import">
                  {isImporting ? <RefreshCw size={14} className="spin" /> : <Plus size={16} />}
                </button>
              </div>
              {importError && (
                <div style={{ color: 'var(--status-err)', fontSize: '11px', display: 'flex', alignItems: 'center', gap: '4px' }}>
                  <ShieldAlert size={12} /> {importError}
                </div>
              )}
              {importSuccess && (
                <div style={{ color: 'var(--status-ok)', fontSize: '11px', display: 'flex', alignItems: 'center', gap: '4px' }}>
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
                <div className="profile-actions">
                  {activeProfileId === p.id && isConnected && (
                    <span className="active-badge">ACTIVE</span>
                  )}
                  <button className="btn-icon-only danger" onClick={(e) => onDeleteProfile(p.id, e)} title="Delete">
                    <Trash2 size={14} />
                  </button>
                </div>
              </div>
            )) : (
              <div style={{ textAlign: 'center', color: 'var(--text-low)', marginTop: '60px' }}>
                <Globe size={40} style={{ strokeWidth: 1, marginBottom: '12px' }} />
                <p style={{ fontSize: '14px' }}>No profiles found</p>
                <p style={{ fontSize: '12px', marginTop: '4px' }}>Import a config using the panel on the right.</p>
              </div>
            )}
          </div>
        </div>

        {/* Detail panel */}
        <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
          {selectedProfileId ? (
            <>
              <div className="flex-row-between" style={{ marginBottom: '16px', flexShrink: 0 }}>
                <h3 style={{ fontSize: '15px', fontWeight: 600 }}>Servers</h3>
                <span style={{ fontSize: '12px', color: 'var(--text-low)' }}>
                  {profiles.find(p => p.id === selectedProfileId)?.name}
                </span>
              </div>
              <div style={{ flex: 1, overflowY: 'auto' }}>
                {profileOutbounds.length > 0 ? (
                  <div className="node-grid">
                    {profileOutbounds.map((node, i) => (
                      <div key={i} className={`node-card ${selectedOutboundTag === node.tag ? 'active' : ''}`} onClick={() => onSelectOutbound(node)}>
                        <span className="node-name" title={node.tag}>{node.tag}</span>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                          <span className="node-type-badge">{node.type}</span>
                          <button className="btn-icon-only" style={{ width: '22px', height: '22px', border: 'none', background: 'transparent', color: 'var(--text-low)' }}
                            onClick={(e) => { e.stopPropagation(); onOpenEditor(node); }} title="Edit node">
                            <Edit3 size={11} />
                          </button>
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <p style={{ fontSize: '13px', color: 'var(--text-low)' }}>No customizable outbounds. All traffic handled by profile rules.</p>
                )}
              </div>
            </>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', flex: 1, color: 'var(--text-low)' }}>
              <Globe size={48} style={{ strokeWidth: 1, marginBottom: '16px', opacity: 0.5 }} />
              <p style={{ fontSize: '14px', fontWeight: 500 }}>No Subscription Selected</p>
              <p style={{ fontSize: '12px', marginTop: '8px', maxWidth: '250px', textAlign: 'center' }}>
                Select a subscription from the left to view and edit its available servers.
              </p>
            </div>
          )}
        </div>
      </div>
    </ViewShell>
  );
}
