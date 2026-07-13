import { Plus, Trash2, Edit3, X, Database, Filter, GitFork, Download, Info } from 'lucide-react';
import { ViewShell } from '../components/ViewShell';
import { useRoutingStore } from '../stores/routingStore';
import { useSettingsStore } from '../stores/settingsStore';
import type { RoutingRuleType, OutboundAction } from '../utils/store';

const BadgeOutbound = ({ action }: { action: OutboundAction }) => {
  let style: React.CSSProperties;
  if (action === 'proxy') {
    style = {
      background: 'var(--text-high)',
      color: 'var(--surface-base)',
      border: '1px solid var(--text-high)',
    };
  } else if (action === 'direct') {
    style = {
      background: 'transparent',
      color: 'var(--text-med)',
      border: '1px solid var(--border-strong)',
    };
  } else if (action === 'block') {
    style = {
      background: 'transparent',
      color: 'var(--text-low)',
      border: '1px dashed var(--border-strong)',
    };
  } else {
    // dns
    style = {
      background: 'var(--surface-sunken)',
      color: 'var(--text-med)',
      border: '1px solid var(--border-default)',
    };
  }

  return (
    <span className="outbound-badge" style={style}>
      {action}
    </span>
  );
};

const Inp = ({ value, onChange, placeholder, mono = false }: {
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  mono?: boolean;
}) => (
  <input
    className="text-input"
    style={mono ? { fontFamily: 'var(--font-mono)', fontSize: '12px' } : undefined}
    value={value}
    onChange={(e) => onChange(e.target.value)}
    placeholder={placeholder}
  />
);

const Sel = ({ value, onChange, options }: {
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
}) => (
  <select className="select-input" value={value} onChange={(e) => onChange(e.target.value)}>
    {options.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
  </select>
);

const Toggle = ({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) => (
  <label className="switch-toggle">
    <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
    <span className="switch-slider"></span>
  </label>
);

export function RoutingView() {
  const {
    routingRules,
    ruleSets,
    showAddRule,
    editingRule,
    ruleForm,
    showAddRuleSet,
    ruleSetForm,
    updatingRuleSet,
    setShowAddRule,
    setEditingRule,
    setRuleForm,
    setShowAddRuleSet,
    setRuleSetForm,
    saveRoutingRule,
    deleteRoutingRule,
    toggleRoutingRuleEnabled,
    saveRuleSet,
    deleteRuleSet,
    updateRuleSet,
  } = useRoutingStore();

  const { settings, updateSettings } = useSettingsStore();

  return (
    <ViewShell
      title="Routing"
      subtitle="Traffic routing engine — GeoIP, GeoSite, rule sets & split tunneling"
      actions={
        <button
          className="btn primary sm"
          onClick={() => {
            setEditingRule(null);
            setRuleForm({ type: 'geoip', value: '', outbound: 'direct', invert: false, notes: '' });
            setShowAddRule(true);
          }}
        >
          <Plus size={14} /> Add Rule
        </button>
      }
    >
      <div className="view-container">
        {/* Global + Rule Sets row */}
        <div className="grid-3">
          {/* Global Settings */}
          <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <GitFork size={16} style={{ color: 'var(--accent-primary)' }} />
              <h3 style={{ fontSize: '14px', fontWeight: 600 }}>Global Routing</h3>
            </div>
            <div className="form-group" style={{ marginBottom: 0 }}>
              <label className="form-label">Final Outbound (Fallback)</label>
              <Sel
                value={settings.finalOutbound}
                onChange={(v) => updateSettings({ finalOutbound: v as OutboundAction })}
                options={[
                  { value: 'proxy', label: '🌐 Proxy (default)' },
                  { value: 'direct', label: '⚡ Direct' },
                  { value: 'block', label: '🚫 Block' },
                ]}
              />
            </div>
            <div className="switch-container" style={{ padding: '10px 14px' }}>
              <div className="switch-details">
                <span className="switch-title" style={{ fontSize: '13px' }}>Bypass LAN / Private IPs</span>
                <span className="switch-desc">Route RFC-1918 addresses directly</span>
              </div>
              <Toggle checked={settings.bypassLan} onChange={(v) => updateSettings({ bypassLan: v })} />
            </div>
          </div>

          {/* Rule Sets */}
          <div className="glass-panel" style={{ gridColumn: 'span 2' }}>
            <div className="flex-row-between" style={{ marginBottom: '16px' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                <Database size={16} style={{ color: 'var(--accent-primary)' }} />
                <h3 style={{ fontSize: '14px', fontWeight: 600 }}>Rule Sets</h3>
                <span className="type-chip">{ruleSets.length}</span>
              </div>
              <button className="btn secondary sm" onClick={() => setShowAddRuleSet(true)}>
                <Plus size={13} /> Add Rule Set
              </button>
            </div>

            {showAddRuleSet && (
              <div className="add-rule-form" style={{ marginBottom: '16px' }}>
                <div className="flex-row-between" style={{ marginBottom: '12px' }}>
                  <span style={{ fontSize: '13px', fontWeight: 600 }}>Add Rule Set</span>
                  <button className="btn-icon-only" style={{ border: 'none', background: 'transparent' }} onClick={() => setShowAddRuleSet(false)}>
                    <X size={14} />
                  </button>
                </div>
                <div style={{ display: 'grid', gridTemplateColumns: '1fr 100px 110px 90px auto', gap: '10px', alignItems: 'end' }}>
                  <div className="form-group" style={{ marginBottom: 0 }}>
                    <label className="form-label">Tag</label>
                    <Inp value={ruleSetForm.tag} onChange={(v) => setRuleSetForm({ tag: v })} placeholder="geoip-cn" mono />
                  </div>
                  <div className="form-group" style={{ marginBottom: 0 }}>
                    <label className="form-label">Type</label>
                    <Sel
                      value={ruleSetForm.type}
                      onChange={(v) => setRuleSetForm({ type: v as 'remote' | 'local' })}
                      options={[{ value: 'remote', label: 'Remote' }, { value: 'local', label: 'Local' }]}
                    />
                  </div>
                  <div className="form-group" style={{ marginBottom: 0 }}>
                    <label className="form-label">Format</label>
                    <Sel
                      value={ruleSetForm.format}
                      onChange={(v) => setRuleSetForm({ format: v as 'binary' | 'source' })}
                      options={[
                        { value: 'binary', label: 'Binary (.srs)' },
                        { value: 'source', label: 'Source (.json)' },
                      ]}
                    />
                  </div>
                  <div className="form-group" style={{ marginBottom: 0 }}>
                    <label className="form-label">Update</label>
                    <Sel
                      value={ruleSetForm.updateInterval}
                      onChange={(v) => setRuleSetForm({ updateInterval: v })}
                      options={[
                        { value: '1h', label: '1h' },
                        { value: '12h', label: '12h' },
                        { value: '1d', label: '1d' },
                        { value: '7d', label: '7d' },
                      ]}
                    />
                  </div>
                  <button className="btn primary" style={{ height: '34px', paddingBottom: 0 }} onClick={saveRuleSet}>Add</button>
                </div>
                {ruleSetForm.type === 'remote' && (
                  <div className="form-group" style={{ marginTop: '10px', marginBottom: 0 }}>
                    <label className="form-label">Remote URL</label>
                    <Inp value={ruleSetForm.url || ''} onChange={(v) => setRuleSetForm({ url: v })} placeholder="https://cdn.jsdelivr.net/.../geoip/cn.srs" />
                  </div>
                )}
              </div>
            )}

            <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', maxHeight: '200px', overflowY: 'auto' }}>
              {ruleSets.map((rs) => (
                <div key={rs.id} style={{ display: 'flex', alignItems: 'center', gap: '10px', padding: '8px 12px', background: 'rgba(0,0,0,0.2)', borderRadius: 'var(--r-sm)' }}>
                  <Database size={12} style={{ color: 'var(--text-low)', flexShrink: 0 }} />
                  <span style={{ fontWeight: 600, fontSize: '13px', color: 'var(--accent-primary)', fontFamily: 'var(--font-mono)' }}>{rs.tag}</span>
                  <span className="type-chip">{rs.type}</span>
                  <span className="type-chip" style={{ background: 'rgba(168,85,247,0.1)', color: '#d8b4fe', border: '1px solid rgba(168,85,247,0.2)' }}>{rs.format}</span>
                  <span className="type-chip">↻ {rs.updateInterval}</span>
                  {rs.lastUpdated && (
                    <span className="type-chip" style={{ background: 'rgba(34,197,94,0.1)', color: '#86efac', border: '1px solid rgba(34,197,94,0.2)' }}>
                      ✓ Cached
                    </span>
                  )}
                  <div style={{ display: 'flex', gap: '6px', marginLeft: 'auto' }}>
                    {rs.type === 'remote' && (
                      <button className="btn-icon-only" style={{ width: '24px', height: '24px' }} onClick={() => updateRuleSet(rs)} disabled={updatingRuleSet === rs.id}>
                        <Download size={11} className={updatingRuleSet === rs.id ? 'spin' : ''} />
                      </button>
                    )}
                    <button className="btn-icon-only danger" style={{ width: '24px', height: '24px' }} onClick={() => deleteRuleSet(rs.id)}>
                      <Trash2 size={11} />
                    </button>
                  </div>
                </div>
              ))}
              {ruleSets.length === 0 && (
                <p style={{ textAlign: 'center', color: 'var(--text-low)', padding: '20px', fontSize: '13px' }}>
                  No rule sets. Add a remote GeoIP/GeoSite rule set above.
                </p>
              )}
            </div>
          </div>
        </div>

        {/* Rules Table */}
        <div className="glass-panel">
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '16px' }}>
            <Filter size={16} style={{ color: 'var(--accent-primary)' }} />
            <h3 style={{ fontSize: '14px', fontWeight: 600 }}>Routing Rules</h3>
            <span className="type-chip">{routingRules.length} rules</span>
          </div>

          {showAddRule && (
            <div className="add-rule-form">
              <div style={{ display: 'grid', gridTemplateColumns: '150px 1fr 130px 70px 1fr auto', gap: '10px', alignItems: 'end' }}>
                <div className="form-group" style={{ marginBottom: 0 }}>
                  <label className="form-label">Rule Type</label>
                  <Sel
                    value={ruleForm.type}
                    onChange={(v) => setRuleForm({ type: v as RoutingRuleType })}
                    options={[
                      { value: 'geoip', label: 'GeoIP' },
                      { value: 'geosite', label: 'GeoSite' },
                      { value: 'domain', label: 'Domain' },
                      { value: 'domain_suffix', label: 'Domain Suffix' },
                      { value: 'domain_keyword', label: 'Domain Keyword' },
                      { value: 'ip_cidr', label: 'IP CIDR' },
                      { value: 'port', label: 'Port' },
                      { value: 'port_range', label: 'Port Range' },
                      { value: 'rule_set', label: 'Rule Set' },
                      { value: 'process_name', label: 'Process Name' },
                    ]}
                  />
                </div>
                <div className="form-group" style={{ marginBottom: 0 }}>
                  <label className="form-label">Value</label>
                  {ruleForm.type === 'rule_set' ? (
                    <Sel
                      value={ruleForm.value}
                      onChange={(v) => setRuleForm({ value: v })}
                      options={[
                        { value: '', label: 'Select Rule Set...' },
                        ...ruleSets.map((rs) => ({ value: rs.tag, label: rs.tag })),
                      ]}
                    />
                  ) : (
                    <Inp
                      value={ruleForm.value}
                      onChange={(v) => setRuleForm({ value: v })}
                      placeholder={ruleForm.type === 'geoip' ? 'cn, private…' : ruleForm.type === 'ip_cidr' ? '10.0.0.0/8' : 'e.g. google.com'}
                      mono
                    />
                  )}
                </div>
                <div className="form-group" style={{ marginBottom: 0 }}>
                  <label className="form-label">Outbound</label>
                  <Sel
                    value={ruleForm.outbound}
                    onChange={(v) => setRuleForm({ outbound: v as OutboundAction })}
                    options={[
                      { value: 'proxy', label: '🌐 Proxy' },
                      { value: 'direct', label: '⚡ Direct' },
                      { value: 'block', label: '🚫 Block' },
                      { value: 'dns', label: '🔍 DNS' },
                    ]}
                  />
                </div>
                <div className="form-group" style={{ marginBottom: 0 }}>
                  <label className="form-label">Invert</label>
                  <div style={{ display: 'flex', justifyContent: 'center', paddingTop: '6px' }}>
                    <Toggle checked={ruleForm.invert} onChange={(v) => setRuleForm({ invert: v })} />
                  </div>
                </div>
                <div className="form-group" style={{ marginBottom: 0 }}>
                  <label className="form-label">Notes</label>
                  <Inp value={ruleForm.notes || ''} onChange={(v) => setRuleForm({ notes: v })} placeholder="e.g. China bypass" />
                </div>
                <div style={{ display: 'flex', gap: '6px', paddingBottom: '1px' }}>
                  <button className="btn primary" style={{ height: '34px' }} onClick={saveRoutingRule}>
                    {editingRule ? 'Update' : 'Add'}
                  </button>
                  <button className="btn secondary" style={{ height: '34px', padding: '0 10px' }} onClick={() => { setShowAddRule(false); setEditingRule(null); }}>
                    <X size={14} />
                  </button>
                </div>
              </div>
            </div>
          )}

          <div className="rules-table">
            <div className="rules-table-header">
              <span style={{ width: '28px' }}>#</span>
              <span style={{ width: '50px' }}>Active</span>
              <span style={{ flex: '0 0 140px' }}>Type</span>
              <span style={{ flex: 1 }}>Value</span>
              <span style={{ flex: '0 0 100px' }}>Outbound</span>
              <span style={{ flex: 1 }}>Notes</span>
              <span style={{ flex: '0 0 60px', textAlign: 'right' }}>Actions</span>
            </div>
            {routingRules.map((rule, i) => (
              <div key={rule.id} className="rules-table-row" style={rule.enabled === false ? { opacity: 0.4 } : undefined}>
                <span style={{ width: '28px', color: 'var(--text-low)', fontSize: '11px' }}>{i + 1}</span>
                <span style={{ width: '50px', display: 'flex', alignItems: 'center' }}>
                  <Toggle
                    checked={rule.enabled !== false}
                    onChange={(checked) => toggleRoutingRuleEnabled(rule.id, checked)}
                  />
                </span>
                <span style={{ flex: '0 0 140px' }}>
                  <span className="type-chip" style={{ fontFamily: 'var(--font-mono)' }}>{rule.type}{rule.invert ? ' !' : ''}</span>
                </span>
                <span style={{ flex: 1, fontFamily: 'var(--font-mono)', fontSize: '12px', color: 'var(--text-high)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{rule.value}</span>
                <span style={{ flex: '0 0 100px' }}><BadgeOutbound action={rule.outbound} /></span>
                <span style={{ flex: 1, fontSize: '11px', color: 'var(--text-low)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{rule.notes || '—'}</span>
                <span style={{ flex: '0 0 60px', display: 'flex', gap: '4px', justifyContent: 'flex-end' }}>
                  <button
                    className="btn-icon-only"
                    style={{ width: '24px', height: '24px' }}
                    onClick={() => { setEditingRule(rule); setShowAddRule(true); }}
                  >
                    <Edit3 size={11} />
                  </button>
                  <button className="btn-icon-only danger" style={{ width: '24px', height: '24px' }} onClick={() => deleteRoutingRule(rule.id)}>
                    <Trash2 size={11} />
                  </button>
                </span>
              </div>
            ))}
            {routingRules.length === 0 && (
              <div style={{ textAlign: 'center', color: 'var(--text-low)', padding: '40px 0', fontSize: '13px' }}>
                No routing rules. All traffic falls to final outbound ({settings.finalOutbound}).
              </div>
            )}
          </div>

          <div className="info-box" style={{ marginTop: '16px' }}>
            <Info size={12} />
            <span>Rules evaluate top-to-bottom. First match wins. Hardcoded: dns → dns-out, private IPs → direct.</span>
          </div>
        </div>
      </div>
    </ViewShell>
  );
}
