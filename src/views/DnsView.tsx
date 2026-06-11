import { Plus, Trash2, Edit3, X, Dna, Server, Filter, Info, ShieldCheck, Database, Ghost } from 'lucide-react';
import { ViewShell } from '../components/ViewShell';
import type { DnsRule, Settings, DnsRuleType, DnsMode, DnsStrategy } from '../utils/store';

const Inp = ({ value, onChange, placeholder, mono = false }: {
  value: string; onChange: (v: string) => void; placeholder?: string; mono?: boolean;
}) => (
  <input className="text-input" style={mono ? { fontFamily: 'var(--font-mono)', fontSize: '12px' } : undefined}
    value={value} onChange={(e) => onChange(e.target.value)} placeholder={placeholder} />
);

const Sel = ({ value, onChange, options }: {
  value: string; onChange: (v: string) => void; options: { value: string; label: string }[];
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

interface DnsViewProps {
  dnsRules: DnsRule[];
  showAddDnsRule: boolean;
  editingDnsRule: DnsRule | null;
  dnsRuleForm: Omit<DnsRule, 'id'>;
  settings: Settings;
  onSetShowAddDnsRule: (v: boolean) => void;
  onSetEditingDnsRule: (r: DnsRule | null) => void;
  onSetDnsRuleForm: (f: Omit<DnsRule, 'id'>) => void;
  onSaveDnsRule: () => void;
  onDeleteDnsRule: (id: string) => void;
  onSaveSettings: (updates: Partial<Settings>) => void;
}

export function DnsView({
  dnsRules, showAddDnsRule, editingDnsRule, dnsRuleForm, settings,
  onSetShowAddDnsRule, onSetEditingDnsRule, onSetDnsRuleForm,
  onSaveDnsRule, onDeleteDnsRule, onSaveSettings,
}: DnsViewProps) {
  const serverColor = (s: string) =>
    s === 'direct' ? 'var(--status-ok)' : s === 'block' ? 'var(--status-err)' : 'var(--accent-primary)';
  const serverBg = (s: string) =>
    s === 'direct' ? 'rgba(34,197,94,0.1)' : s === 'block' ? 'rgba(239,68,68,0.1)' : 'rgba(74,158,255,0.1)';

  return (
    <ViewShell
      title="DNS"
      subtitle="DNS engine — DoH, DoT, DoQ, FakeIP, DNS rules & leak protection"
      actions={
        <button className="btn primary sm" onClick={() => {
          onSetEditingDnsRule(null);
          onSetDnsRuleForm({ type: 'geosite', value: '', server: 'direct', disableCache: false, invert: false });
          onSetShowAddDnsRule(true);
        }}>
          <Plus size={14} /> Add Rule
        </button>
      }
    >
      <div className="view-container">
        {/* Config row */}
        <div className="grid-2">
          {/* Mode & Options */}
          <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <Dna size={16} style={{ color: 'var(--accent-primary)' }} />
              <h3 style={{ fontSize: '14px', fontWeight: 600 }}>DNS Mode & Options</h3>
            </div>
            <div className="form-group" style={{ marginBottom: 0 }}>
              <label className="form-label">Resolution Mode</label>
              <Sel value={settings.dnsMode} onChange={(v) => onSaveSettings({ dnsMode: v as DnsMode })}
                options={[
                  { value: 'fakeip', label: '👻 FakeIP (Recommended for TUN)' },
                  { value: 'normal', label: '🔍 Normal (Standard DNS)' },
                ]} />
            </div>
            <div className="form-group" style={{ marginBottom: 0 }}>
              <label className="form-label">IP Strategy</label>
              <Sel value={settings.dnsStrategy} onChange={(v) => onSaveSettings({ dnsStrategy: v as DnsStrategy })}
                options={[
                  { value: 'prefer_ipv4', label: 'Prefer IPv4' }, { value: 'prefer_ipv6', label: 'Prefer IPv6' },
                  { value: 'ipv4_only', label: 'IPv4 Only' }, { value: 'ipv6_only', label: 'IPv6 Only' },
                ]} />
            </div>
            {[
              { key: 'dnsLeakProtection', title: 'DNS Leak Protection', desc: 'Force queries through tunnel', icon: ShieldCheck },
              { key: 'dnsCaching', title: 'DNS Response Caching', desc: 'Cache responses to reduce latency', icon: Database },
            ].map(({ key, title, desc, icon: Icon }) => (
              <div key={key} className="switch-container" style={{ padding: '10px 14px' }}>
                <div className="switch-details">
                  <span className="switch-title" style={{ fontSize: '13px', display: 'flex', alignItems: 'center', gap: '6px' }}>
                    <Icon size={12} style={{ color: 'var(--accent-primary)' }} /> {title}
                  </span>
                  <span className="switch-desc">{desc}</span>
                </div>
                <Toggle checked={(settings as any)[key]} onChange={(v) => onSaveSettings({ [key]: v } as any)} />
              </div>
            ))}
          </div>

          {/* DNS Servers */}
          <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <Server size={16} style={{ color: 'var(--accent-primary)' }} />
              <h3 style={{ fontSize: '14px', fontWeight: 600 }}>DNS Servers</h3>
            </div>
            {[
              { key: 'primaryDns', label: 'Primary DNS (Encrypted)', placeholder: 'https://1.1.1.1/dns-query', badge: 'DoH', color: 'var(--accent-primary)' },
              { key: 'fallbackDns', label: 'Fallback DNS', placeholder: 'https://8.8.8.8/dns-query', badge: 'DoH', color: 'var(--accent-secondary)' },
              { key: 'directDns', label: 'Direct DNS (ISP / Router)', placeholder: '192.168.1.1', badge: 'UDP', color: 'var(--status-ok)' },
            ].map(({ key, label, placeholder, badge, color }) => (
              <div key={key} className="form-group" style={{ marginBottom: 0 }}>
                <div className="flex-row-between">
                  <label className="form-label">{label}</label>
                  <span style={{ fontSize: '10px', fontWeight: 700, padding: '2px 6px', borderRadius: '4px', background: `${color}18`, color, border: `1px solid ${color}40` }}>{badge}</span>
                </div>
                <Inp value={(settings as any)[key]} onChange={(v) => onSaveSettings({ [key]: v } as any)} placeholder={placeholder} mono />
              </div>
            ))}
            <div className="info-box">
              <Info size={12} />
              <span><code>https://</code> = DoH · <code>tls://</code> = DoT · <code>quic://</code> = DoQ · plain IP = UDP</span>
            </div>
          </div>
        </div>

        {/* FakeIP config */}
        {settings.dnsMode === 'fakeip' && (
          <div className="glass-panel">
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '16px' }}>
              <Ghost size={16} style={{ color: 'var(--accent-secondary)' }} />
              <h3 style={{ fontSize: '14px', fontWeight: 600 }}>FakeIP Configuration</h3>
            </div>
            <div className="grid-2">
              <div className="form-group" style={{ marginBottom: 0 }}>
                <label className="form-label">FakeIP CIDR Range</label>
                <Inp value={settings.fakeipRange} onChange={(v) => onSaveSettings({ fakeipRange: v })} placeholder="198.18.0.0/15" mono />
                <span style={{ fontSize: '11px', color: 'var(--text-low)', marginTop: '4px', display: 'block' }}>Must not overlap your LAN.</span>
              </div>
              <div className="form-group" style={{ marginBottom: 0 }}>
                <label className="form-label">FakeIP Filter (bypass)</label>
                <Inp value={settings.fakeipFilter} onChange={(v) => onSaveSettings({ fakeipFilter: v })} placeholder="geosite:private" mono />
                <span style={{ fontSize: '11px', color: 'var(--text-low)', marginTop: '4px', display: 'block' }}>Domains excluded from FakeIP.</span>
              </div>
            </div>
          </div>
        )}

        {/* DNS Rules */}
        <div className="glass-panel">
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '16px' }}>
            <Filter size={16} style={{ color: 'var(--accent-primary)' }} />
            <h3 style={{ fontSize: '14px', fontWeight: 600 }}>DNS Rules</h3>
            <span className="type-chip">{dnsRules.length} rules</span>
          </div>

          {showAddDnsRule && (
            <div className="add-rule-form">
              <div style={{ display: 'grid', gridTemplateColumns: '150px 1fr 160px 70px auto', gap: '10px', alignItems: 'end' }}>
                <div className="form-group" style={{ marginBottom: 0 }}>
                  <label className="form-label">Type</label>
                  <Sel value={dnsRuleForm.type} onChange={(v) => onSetDnsRuleForm({ ...dnsRuleForm, type: v as DnsRuleType })}
                    options={[
                      { value: 'geosite', label: 'GeoSite' }, { value: 'domain', label: 'Domain' },
                      { value: 'domain_suffix', label: 'Domain Suffix' }, { value: 'domain_keyword', label: 'Keyword' },
                      { value: 'rule_set', label: 'Rule Set' }, { value: 'ip_cidr', label: 'IP CIDR' },
                    ]} />
                </div>
                <div className="form-group" style={{ marginBottom: 0 }}>
                  <label className="form-label">Value</label>
                  <Inp value={dnsRuleForm.value} onChange={(v) => onSetDnsRuleForm({ ...dnsRuleForm, value: v })} placeholder="cn, private, google.com…" mono />
                </div>
                <div className="form-group" style={{ marginBottom: 0 }}>
                  <label className="form-label">Use DNS Server</label>
                  <Sel value={dnsRuleForm.server} onChange={(v) => onSetDnsRuleForm({ ...dnsRuleForm, server: v })}
                    options={[
                      { value: 'primary', label: 'Primary (Encrypted)' }, { value: 'fallback', label: 'Fallback' },
                      { value: 'direct', label: 'Direct (ISP)' }, { value: 'block', label: 'Block (NXDOMAIN)' },
                    ]} />
                </div>
                <div className="form-group" style={{ marginBottom: 0 }}>
                  <label className="form-label">No Cache</label>
                  <div style={{ paddingTop: '6px', display: 'flex', justifyContent: 'center' }}>
                    <Toggle checked={dnsRuleForm.disableCache} onChange={(v) => onSetDnsRuleForm({ ...dnsRuleForm, disableCache: v })} />
                  </div>
                </div>
                <div style={{ display: 'flex', gap: '6px', paddingBottom: '1px' }}>
                  <button className="btn primary" style={{ height: '34px' }} onClick={onSaveDnsRule}>
                    {editingDnsRule ? 'Update' : 'Add'}
                  </button>
                  <button className="btn secondary" style={{ height: '34px', padding: '0 10px' }} onClick={() => { onSetShowAddDnsRule(false); onSetEditingDnsRule(null); }}>
                    <X size={14} />
                  </button>
                </div>
              </div>
            </div>
          )}

          <div className="rules-table">
            <div className="rules-table-header">
              <span style={{ width: '28px' }}>#</span>
              <span style={{ flex: '0 0 140px' }}>Type</span>
              <span style={{ flex: 1 }}>Value</span>
              <span style={{ flex: '0 0 140px' }}>Server</span>
              <span style={{ flex: '0 0 70px' }}>Cache</span>
              <span style={{ flex: '0 0 60px', textAlign: 'right' }}>Actions</span>
            </div>
            {dnsRules.map((rule, i) => (
              <div key={rule.id} className="rules-table-row">
                <span style={{ width: '28px', color: 'var(--text-low)', fontSize: '11px' }}>{i + 1}</span>
                <span style={{ flex: '0 0 140px' }}><span className="type-chip" style={{ fontFamily: 'var(--font-mono)' }}>{rule.type}</span></span>
                <span style={{ flex: 1, fontFamily: 'var(--font-mono)', fontSize: '12px', color: 'var(--text-high)' }}>{rule.value}</span>
                <span style={{ flex: '0 0 140px' }}>
                  <span className="type-chip" style={{ background: serverBg(rule.server), color: serverColor(rule.server), border: `1px solid ${serverColor(rule.server)}30` }}>
                    {rule.server}
                  </span>
                </span>
                <span style={{ flex: '0 0 70px', fontSize: '12px', color: rule.disableCache ? 'var(--status-err)' : 'var(--status-ok)' }}>
                  {rule.disableCache ? '✗ off' : '✓ on'}
                </span>
                <span style={{ flex: '0 0 60px', display: 'flex', gap: '4px', justifyContent: 'flex-end' }}>
                  <button className="btn-icon-only" style={{ width: '24px', height: '24px' }}
                    onClick={() => { onSetEditingDnsRule(rule); onSetDnsRuleForm({ type: rule.type, value: rule.value, server: rule.server, disableCache: rule.disableCache, invert: rule.invert }); onSetShowAddDnsRule(true); }}>
                    <Edit3 size={11} />
                  </button>
                  <button className="btn-icon-only danger" style={{ width: '24px', height: '24px' }} onClick={() => onDeleteDnsRule(rule.id)}>
                    <Trash2 size={11} />
                  </button>
                </span>
              </div>
            ))}
            {dnsRules.length === 0 && (
              <div style={{ textAlign: 'center', color: 'var(--text-low)', padding: '40px 0', fontSize: '13px' }}>
                No DNS rules. All domains resolve via the primary DNS server.
              </div>
            )}
          </div>
        </div>
      </div>
    </ViewShell>
  );
}
