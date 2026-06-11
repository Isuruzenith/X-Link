import React from 'react';
import { Globe, Trash2, Plus, RefreshCw, Check, ShieldAlert, FolderOpen, Clipboard } from 'lucide-react';
import { ViewShell } from '../components/ViewShell';
import type { Profile } from '../utils/store';

interface ProfilesViewProps {
  profiles: Profile[];
  selectedProfileId: string | null;
  activeProfileId: string | null;
  isConnected: boolean;
  importTab: 'url' | 'file' | 'clipboard';
  importName: string;
  importUrl: string;
  importContent: string;
  importFilePath: string;
  importError: string | null;
  importSuccess: boolean;
  isImporting: boolean;
  onSelectProfile: (id: string) => void;
  onDeleteProfile: (id: string, e: React.MouseEvent) => void;
  onSetImportTab: (tab: 'url' | 'file' | 'clipboard') => void;
  onSetImportName: (v: string) => void;
  onSetImportUrl: (v: string) => void;
  onSetImportContent: (v: string) => void;
  onPickFile: () => void;
  onPasteClipboard: () => void;
  onImportProfile: (e: React.FormEvent) => void;
}

export function ProfilesView({
  profiles, selectedProfileId, activeProfileId, isConnected,
  importTab, importName, importUrl, importContent, importFilePath,
  importError, importSuccess, isImporting,
  onSelectProfile, onDeleteProfile, onSetImportTab, onSetImportName,
  onSetImportUrl, onSetImportContent, onPickFile, onPasteClipboard, onImportProfile,
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

          {/* Segmented control */}
          <div className="seg-control" style={{ marginBottom: '20px', flexShrink: 0 }}>
            {(['url', 'file', 'clipboard'] as const).map((tab) => (
              <div key={tab} className={`seg-item ${importTab === tab ? 'active' : ''}`} onClick={() => onSetImportTab(tab)}>
                {tab === 'url' ? 'Subscription URL' : tab === 'file' ? 'Local File' : 'Clipboard'}
              </div>
            ))}
          </div>

          <form onSubmit={onImportProfile} style={{ display: 'flex', flexDirection: 'column', gap: '16px', flex: 1 }}>
            <div className="form-group" style={{ marginBottom: 0 }}>
              <label className="form-label">Profile Name</label>
              <input className="text-input" value={importName} onChange={(e) => onSetImportName(e.target.value)} placeholder="e.g. Premium Proxy" />
            </div>

            {importTab === 'url' && (
              <div className="form-group" style={{ marginBottom: 0 }}>
                <label className="form-label">Subscription Link</label>
                <input type="url" className="text-input" value={importUrl} onChange={(e) => onSetImportUrl(e.target.value)} placeholder="https://provider.com/clash.yaml" />
              </div>
            )}

            {importTab === 'file' && (
              <div className="form-group" style={{ marginBottom: 0 }}>
                <label className="form-label">Configuration File</label>
                <div style={{ display: 'flex', gap: '10px' }}>
                  <input className="text-input" value={importFilePath} onChange={() => {}} placeholder="Select JSON, YAML, or Base64 file..." readOnly />
                  <button type="button" className="btn secondary" onClick={onPickFile}><FolderOpen size={16} /></button>
                </div>
              </div>
            )}

            {importTab === 'clipboard' && (
              <div className="form-group" style={{ marginBottom: 0, flex: 1, display: 'flex', flexDirection: 'column' }}>
                <div className="flex-row-between">
                  <label className="form-label">Raw Config (Clash YAML or base64)</label>
                  <button type="button" className="btn secondary sm" onClick={onPasteClipboard}>
                    <Clipboard size={12} /> Paste
                  </button>
                </div>
                <textarea className="text-input" style={{ flex: 1, minHeight: '140px', fontFamily: 'var(--font-mono)', fontSize: '12px', resize: 'none', marginTop: '8px' }}
                  value={importContent} onChange={(e) => onSetImportContent(e.target.value)}
                  placeholder="Paste subscription bytes, Clash YAML, or sing-box JSON here..." />
              </div>
            )}

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
