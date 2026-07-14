import { Plus, Trash2, Edit3, X, Database, Filter, GitFork, Download, Info } from 'lucide-react';
import { ViewShell } from '../components/ViewShell';
import { useRoutingStore } from '../stores/routingStore';
import { useSettingsStore } from '../stores/settingsStore';
import type { RoutingRuleType, OutboundAction } from '../utils/store';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { FormInput, FormSelect, FormSwitchRow } from '@/components/form';
import { Switch } from '@/components/ui/switch';
import { cn } from '@/lib/utils';

const BadgeOutbound = ({ action }: { action: OutboundAction }) => {
  const variantMap: Record<OutboundAction, string> = {
    proxy: 'bg-primary text-primary-foreground border-primary',
    direct: 'bg-transparent text-muted-foreground border-border',
    block: 'bg-transparent text-muted-foreground border-dashed border-border',
    dns: 'bg-muted text-muted-foreground border-border',
  };

  return (
    <Badge variant="outline" className={cn('text-2xs font-bold uppercase tracking-wide', variantMap[action])}>
      {action}
    </Badge>
  );
};

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
        <Button
          size="sm"
          className="h-8 gap-1 px-3 text-xs font-semibold"
          onClick={() => {
            setEditingRule(null);
            setRuleForm({ type: 'geoip', value: '', outbound: 'direct', invert: false, notes: '' });
            setShowAddRule(true);
          }}
        >
          <Plus className="size-3.5" /> Add Rule
        </Button>
      }
    >
      <div className="flex flex-col gap-6 overflow-hidden flex-1 min-h-0">
        {/* Global + Rule Sets row */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          {/* Global Settings */}
          <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-4">
            <CardHeader className="p-0 flex flex-row items-center gap-2 space-y-0 pb-1">
              <GitFork className="size-4 text-muted-foreground" />
              <CardTitle className="text-xs font-bold text-foreground">Global Routing</CardTitle>
            </CardHeader>
            <CardContent className="p-0 flex flex-col gap-3.5">
              <div className="flex flex-col gap-1.5">
                <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider">Final Outbound (Fallback)</label>
                <FormSelect
                  value={settings.finalOutbound}
                  onChange={(v) => updateSettings({ finalOutbound: v as OutboundAction })}
                  options={[
                    { value: 'proxy', label: '🌐 Proxy (default)' },
                    { value: 'direct', label: '⚡ Direct' },
                    { value: 'block', label: '🚫 Block' },
                  ]}
                />
              </div>
              <FormSwitchRow
                title="Bypass LAN / Private IPs"
                desc="Route RFC-1918 addresses directly"
                checked={settings.bypassLan}
                onChange={(v) => updateSettings({ bypassLan: v })}
              />
            </CardContent>
          </Card>

          {/* Rule Sets */}
          <Card className="col-span-1 md:col-span-2 p-4 bg-card border-border shadow-sm flex flex-col overflow-hidden">
            <CardHeader className="p-0 pb-3 flex flex-row items-center justify-between space-y-0 shrink-0">
              <div className="flex items-center gap-2">
                <Database className="size-4 text-muted-foreground" />
                <CardTitle className="text-xs font-bold text-foreground">Rule Sets</CardTitle>
                <Badge variant="outline" className="h-4 px-1.5 text-[9px] border-border bg-muted/30">
                  {ruleSets.length}
                </Badge>
              </div>
              <Button
                variant="outline"
                size="sm"
                className="h-6 gap-1 px-2 text-[10px] font-semibold"
                onClick={() => setShowAddRuleSet(true)}
              >
                <Plus className="size-3" /> Add Rule Set
              </Button>
            </CardHeader>

            <CardContent className="p-0 flex-1 flex flex-col overflow-hidden">
              {showAddRuleSet && (
                <div className="p-3 mb-3 bg-muted/40 border border-border/50 rounded-lg shrink-0">
                  <div className="flex justify-between items-center mb-2">
                    <span className="text-xs font-bold text-foreground">Add Rule Set</span>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="size-6 text-muted-foreground hover:text-foreground"
                      onClick={() => setShowAddRuleSet(false)}
                    >
                      <X className="size-3.5" />
                    </Button>
                  </div>
                  <div className="grid grid-cols-2 sm:grid-cols-5 gap-2 items-end">
                    <div className="flex flex-col gap-1 col-span-2 sm:col-span-1">
                      <label className="text-[9.5px] font-bold text-muted-foreground uppercase">Tag</label>
                      <FormInput value={ruleSetForm.tag} onChange={(v) => setRuleSetForm({ tag: v })} placeholder="geoip-cn" mono />
                    </div>
                    <div className="flex flex-col gap-1">
                      <label className="text-[9.5px] font-bold text-muted-foreground uppercase">Type</label>
                      <FormSelect
                        value={ruleSetForm.type}
                        onChange={(v) => setRuleSetForm({ type: v as 'remote' | 'local' })}
                        options={[{ value: 'remote', label: 'Remote' }, { value: 'local', label: 'Local' }]}
                      />
                    </div>
                    <div className="flex flex-col gap-1">
                      <label className="text-[9.5px] font-bold text-muted-foreground uppercase">Format</label>
                      <FormSelect
                        value={ruleSetForm.format}
                        onChange={(v) => setRuleSetForm({ format: v as 'binary' | 'source' })}
                        options={[
                          { value: 'binary', label: 'Binary (.srs)' },
                          { value: 'source', label: 'Source (.json)' },
                        ]}
                      />
                    </div>
                    <div className="flex flex-col gap-1">
                      <label className="text-[9.5px] font-bold text-muted-foreground uppercase">Update</label>
                      <FormSelect
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
                    <Button
                      variant="default"
                      size="sm"
                      className="h-8 col-span-2 sm:col-span-1 font-semibold"
                      onClick={saveRuleSet}
                    >
                      Add
                    </Button>
                  </div>
                  {ruleSetForm.type === 'remote' && (
                    <div className="flex flex-col gap-1 mt-2.5">
                      <label className="text-[9.5px] font-bold text-muted-foreground uppercase">Remote URL</label>
                      <FormInput value={ruleSetForm.url || ''} onChange={(v) => setRuleSetForm({ url: v })} placeholder="https://cdn.jsdelivr.net/.../geoip/cn.srs" />
                    </div>
                  )}
                </div>
              )}

              <div className="flex flex-col gap-1.5 overflow-y-auto max-h-[170px] pr-1">
                {ruleSets.map((rs) => (
                  <div key={rs.id} className="flex items-center gap-2 p-2 bg-muted/40 border border-border/40 rounded-lg min-w-0">
                    <Database className="size-3 text-muted-foreground shrink-0" />
                    <span className="font-mono text-xs font-bold text-foreground truncate min-w-0 pr-1">{rs.tag}</span>
                    <Badge variant="secondary" className="h-4 px-1.5 text-[8.5px] font-semibold tracking-wider bg-background/50 border border-border/40 text-muted-foreground shrink-0">{rs.type}</Badge>
                    <Badge variant="outline" className="h-4 px-1.5 text-[8.5px] font-semibold tracking-wider border-border bg-background/30 text-muted-foreground shrink-0">{rs.format}</Badge>
                    <Badge variant="outline" className="h-4 px-1.5 text-[8.5px] font-normal border-border/40 text-muted-foreground/80 shrink-0">↻ {rs.updateInterval}</Badge>
                    {rs.lastUpdated && (
                      <Badge className="h-4 px-1.5 text-[8.5px] font-bold bg-foreground text-background shrink-0">
                        ✓ Cached
                      </Badge>
                    )}
                    <div className="flex gap-1 ml-auto shrink-0 pl-1.5">
                      {rs.type === 'remote' && (
                        <Button
                          variant="ghost"
                          size="icon"
                          className="size-6 text-muted-foreground hover:text-foreground rounded"
                          onClick={() => updateRuleSet(rs)}
                          disabled={updatingRuleSet === rs.id}
                        >
                          <Download className={`size-3 ${updatingRuleSet === rs.id ? 'animate-spin' : ''}`} />
                        </Button>
                      )}
                      <Button
                        variant="ghost"
                        size="icon"
                        className="size-6 text-muted-foreground hover:text-destructive hover:bg-destructive/10 rounded"
                        onClick={() => deleteRuleSet(rs.id)}
                      >
                        <Trash2 className="size-3" />
                      </Button>
                    </div>
                  </div>
                ))}
                {ruleSets.length === 0 && (
                  <p className="text-center text-muted-foreground py-6 text-[11px] leading-relaxed">
                    No rule sets. Add a remote GeoIP/GeoSite rule set above.
                  </p>
                )}
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Rules Table */}
        <Card className="p-4 bg-card border-border shadow-sm flex flex-col overflow-hidden flex-1 min-h-0">
          <CardHeader className="p-0 pb-3 flex flex-row items-center gap-2 space-y-0 shrink-0">
            <Filter className="size-4 text-muted-foreground" />
            <CardTitle className="text-xs font-bold text-foreground">Routing Rules</CardTitle>
            <Badge variant="outline" className="h-4 px-1.5 text-[9px] border-border bg-muted/30">
              {routingRules.length} rules
            </Badge>
          </CardHeader>

          <CardContent className="p-0 flex-1 flex flex-col overflow-hidden min-h-0">
            {showAddRule && (
              <div className="p-3 mb-3.5 bg-muted/40 border border-border/50 rounded-lg shrink-0">
                <div className="grid grid-cols-2 sm:grid-cols-6 gap-2.5 items-end">
                  <div className="flex flex-col gap-1">
                    <label className="text-[9.5px] font-bold text-muted-foreground uppercase">Rule Type</label>
                    <FormSelect
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
                  <div className="flex flex-col gap-1 col-span-2 sm:col-span-1">
                    <label className="text-[9.5px] font-bold text-muted-foreground uppercase">Value</label>
                    {ruleForm.type === 'rule_set' ? (
                      <FormSelect
                        value={ruleForm.value}
                        onChange={(v) => setRuleForm({ value: v })}
                        options={[
                          { value: '', label: 'Select Set...' },
                          ...ruleSets.map((rs) => ({ value: rs.tag, label: rs.tag })),
                        ]}
                      />
                    ) : (
                      <FormInput
                        value={ruleForm.value}
                        onChange={(v) => setRuleForm({ value: v })}
                        placeholder={ruleForm.type === 'geoip' ? 'cn, private…' : ruleForm.type === 'ip_cidr' ? '10.0.0.0/8' : 'e.g. google.com'}
                        mono
                      />
                    )}
                  </div>
                  <div className="flex flex-col gap-1">
                    <label className="text-[9.5px] font-bold text-muted-foreground uppercase">Outbound</label>
                    <FormSelect
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
                  <div className="flex flex-col gap-1 items-center">
                    <label className="text-[9.5px] font-bold text-muted-foreground uppercase">Invert</label>
                    <div className="h-8 flex items-center">
                      <Switch checked={ruleForm.invert} onCheckedChange={(v) => setRuleForm({ invert: v })} />
                    </div>
                  </div>
                  <div className="flex flex-col gap-1">
                    <label className="text-[9.5px] font-bold text-muted-foreground uppercase">Notes</label>
                    <FormInput value={ruleForm.notes || ''} onChange={(v) => setRuleForm({ notes: v })} placeholder="e.g. China bypass" />
                  </div>
                  <div className="flex gap-1 shrink-0 pb-0.5 justify-end">
                    <Button
                      variant="default"
                      size="sm"
                      className="h-8 font-semibold text-xs px-3"
                      onClick={saveRoutingRule}
                    >
                      {editingRule ? 'Update' : 'Add'}
                    </Button>
                    <Button
                      variant="outline"
                      size="icon"
                      className="size-8"
                      onClick={() => { setShowAddRule(false); setEditingRule(null); }}
                    >
                      <X className="size-4" />
                    </Button>
                  </div>
                </div>
              </div>
            )}

            <div className="flex-1 overflow-hidden flex flex-col min-h-0">
              <div className="rules-table border border-border/40 rounded-lg overflow-hidden bg-muted/10 flex-1 flex flex-col min-h-0">
                <div className="rules-table-header bg-muted/40 border-b border-border/60 text-[10px] uppercase font-bold tracking-wider text-muted-foreground py-2.5 px-3 flex items-center shrink-0">
                  <span className="w-7">#</span>
                  <span className="w-12">Active</span>
                  <span className="w-32 shrink-0">Type</span>
                  <span className="flex-1 truncate">Value</span>
                  <span className="w-24 shrink-0">Outbound</span>
                  <span className="flex-1 truncate">Notes</span>
                  <span className="w-16 shrink-0 text-right">Actions</span>
                </div>
                <div className="flex-1 overflow-y-auto flex flex-col divide-y divide-border/30">
                  {routingRules.map((rule, i) => (
                    <div
                      key={rule.id}
                      className={`flex items-center text-xs py-2 px-3 hover:bg-muted/20 transition-all ${
                        rule.enabled === false ? 'opacity-40' : ''
                      }`}
                    >
                      <span className="w-7 text-[10px] text-muted-foreground font-mono">{i + 1}</span>
                      <span className="w-12 flex items-center">
                        <Switch
                          checked={rule.enabled !== false}
                          onCheckedChange={(checked) => toggleRoutingRuleEnabled(rule.id, checked)}
                        />
                      </span>
                      <span className="w-32 shrink-0 pr-1">
                        <Badge variant="outline" className="h-4.5 px-1.5 text-[9px] font-mono border-border bg-background/50 font-normal">
                          {rule.type}{rule.invert ? ' !' : ''}
                        </Badge>
                      </span>
                      <span className="flex-1 font-mono text-[11px] text-foreground truncate min-w-0 pr-1" title={rule.value}>{rule.value}</span>
                      <span className="w-24 shrink-0 pr-1"><BadgeOutbound action={rule.outbound} /></span>
                      <span className="flex-1 text-[11px] text-muted-foreground truncate min-w-0 pr-1" title={rule.notes || ''}>{rule.notes || '—'}</span>
                      <span className="w-16 shrink-0 flex gap-1 justify-end pl-1">
                        <Button
                          variant="ghost"
                          size="icon"
                          className="size-6 text-muted-foreground hover:text-foreground rounded"
                          onClick={() => { setEditingRule(rule); setShowAddRule(true); }}
                        >
                          <Edit3 className="size-3" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="size-6 text-muted-foreground hover:text-destructive hover:bg-destructive/10 rounded"
                          onClick={() => deleteRoutingRule(rule.id)}
                        >
                          <Trash2 className="size-3" />
                        </Button>
                      </span>
                    </div>
                  ))}
                  {routingRules.length === 0 && (
                    <div className="text-center text-muted-foreground py-8 text-xs">
                      No routing rules. All traffic falls to final outbound ({settings.finalOutbound}).
                    </div>
                  )}
                </div>
              </div>
            </div>

            <div className="info-box mt-3 bg-muted/40 border border-border/40 p-2.5 px-3 rounded-lg flex items-start gap-2 text-[10.5px] text-muted-foreground leading-relaxed shrink-0">
              <Info className="size-3.5 mt-0.5 text-foreground shrink-0" />
              <span>Rules evaluate top-to-bottom. First match wins. Hardcoded: dns → dns-out, private IPs → direct.</span>
            </div>
          </CardContent>
        </Card>
      </div>
    </ViewShell>
  );
}
