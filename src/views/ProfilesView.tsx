import React from 'react';
import { Globe, Trash2, Plus, RefreshCw, Check, ShieldAlert, Clipboard } from 'lucide-react';
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
  onSelectProfile: (id: string) => void;
  onDeleteProfile: (id: string, e: React.MouseEvent) => void;
  onSetImportName: (v: string) => void;
  onSetImportContent: (v: string) => void;
  onPasteClipboard: () => void;
  onImportProfile: (e: React.FormEvent) => void;
}

export function ProfilesView({
  profiles, selectedProfileId, activeProfileId, isConnected,
  importName, importContent, importError, importSuccess, isImporting,
  onSelectProfile, onDeleteProfile, onSetImportName, onSetImportContent,
  onPasteClipboard, onImportProfile,
}: ProfilesViewProps) {
  return (
    <ViewShell title="Profiles" subtitle="Import and manage proxy subscription profiles">
      <div className="grid-2" style={{ height: 'calc(100vh - 100px)' }}>
        {/* Profile list */}
        <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
          <h3 style={{ fontSize: '14px', fontWeight: 600, marginBottom: '16px', flexShrink: 0 }}>Your Configs</h3>
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

        {/* Import panel */}
        <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
          <h3 style={{ fontSize: '14px', fontWeight: 600, marginBottom: '16px', flexShrink: 0 }}>Import Profile</h3>

          <form onSubmit={onImportProfile} style={{ display: 'flex', flexDirection: 'column', gap: '16px', flex: 1 }}>
            <div className="form-group" style={{ marginBottom: 0 }}>
              <label className="form-label">Profile Name</label>
              <input className="text-input" value={importName} onChange={(e) => onSetImportName(e.target.value)} placeholder="e.g. Premium Proxy (leave empty to auto-detect)" />
            </div>

            <div className="form-group" style={{ marginBottom: 0, flex: 1, display: 'flex', flexDirection: 'column' }}>
              <div className="flex-row-between">
                <label className="form-label">Share Link / Raw Config</label>
                <button type="button" className="btn secondary sm" onClick={onPasteClipboard}>
                  <Clipboard size={12} /> Paste
                </button>
              </div>
              <textarea className="text-input" style={{ flex: 1, minHeight: '140px', fontFamily: 'var(--font-mono)', fontSize: '12px', resize: 'none', marginTop: '8px' }}
                value={importContent} onChange={(e) => onSetImportContent(e.target.value)}
                placeholder="Paste vless://, vmess://, trojan://, ss:// link or Clash YAML / sing-box JSON here..." />
            </div>

            {importError && (
              <div className="alert-box error"><ShieldAlert size={14} /><span>{importError}</span></div>
            )}
            {importSuccess && (
              <div className="alert-box success"><Check size={14} /><span>Profile imported and validated!</span></div>
            )}

            <button type="submit" className="btn primary" disabled={isImporting}
              style={{ marginTop: 'auto', width: '100%', height: '44px' }}>
              {isImporting ? <><RefreshCw size={16} className="spin" /> Validating…</> : <><Plus size={18} /> Import & Validate</>}
            </button>
          </form>
        </div>
      </div>
    </ViewShell>
  );
}
