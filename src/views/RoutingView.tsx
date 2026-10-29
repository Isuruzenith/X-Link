import { Icon } from '@/components/ui/icon';
import { ViewShell } from '../components/ViewShell';
import { useRoutingStore } from '../stores/routingStore';
import { useSettingsStore } from '../stores/settingsStore';
import { useConnectionStore } from '../stores/connectionStore';
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
  const isConnected = useConnectionStore((state) => state.connectionStatus === 'connected');

  return (
    <ViewShell
      title="Routing"
      subtitle="Set the final outbound, manage rule sets and choose how traffic is routed"
      actions={
        <Button
          size="sm"
          className="font-semibold"
          onClick={() => {
            setEditingRule(null);
            setRuleForm({ type: 'geoip', value: '', outbound: 'direct', invert: false, notes: '' });
            setShowAddRule(true);
          }}
        >
          <Icon name="add" size={14} /> Add Rule
        </Button>
      }
    >
      <div className="utility-view routing-view">
        {isConnected && (
          <div className="shrink-0 rounded border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-[11px] text-amber-600 dark:text-amber-400">
            Routing changes are saved and applied with a validated restart of the running proxy; if the restart fails the previous rules are restored.
          </div>
        )}
        {/* Global + Rule Sets row */}
        <div className="routing-overview">
          {/* Global Settings */}
          <Card className="p-4 bg-card border-border shadow-sm flex flex-col gap-4">
            <CardHeader className="p-0 flex flex-row items-center gap-2 space-y-0 pb-1">
              <Icon name="alt_route" size={16} className="text-muted-foreground" />
              <CardTitle className="text-xs font-bold text-foreground">Global Routing</CardTitle>
            </CardHeader>
            <CardContent className="p-0 flex flex-col gap-3.5">
              <div className="flex flex-col gap-1.5">
                <FormSelect
                  label="Final outbound (fallback)"
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
          <Card className="routing-rule-sets p-4 bg-card flex flex-col min-w-0">
            <CardHeader className="p-0 pb-3 flex flex-row items-center justify-between space-y-0 shrink-0">
              <div className="flex items-center gap-2">
                <Icon name="database" size={16} className="text-muted-foreground" />
                <CardTitle className="text-xs font-bold text-foreground">Rule Sets</CardTitle>
                <Badge variant="outline" className="h-4 px-1.5 text-[9px] border-border bg-muted/30">
                  {ruleSets.length}
                </Badge>
              </div>
              <Button
                variant="outline"
                size="sm"
                className="font-semibold"
                onClick={() => setShowAddRuleSet(true)}
              >
                <Icon name="add" size={12} /> Add Rule Set
              </Button>
            </CardHeader>

            <CardContent className="p-0 flex-1 flex flex-col min-w-0">
              {showAddRuleSet && (
                <div className="routing-rule-set-form p-3 mb-3 bg-muted/40 rounded-lg shrink-0">
                  <div className="flex justify-between items-center mb-2">
                    <span className="text-xs font-bold text-foreground">Add Rule Set</span>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="size-6 text-muted-foreground hover:text-foreground"
                      onClick={() => setShowAddRuleSet(false)}
                      aria-label="Cancel adding rule set"
                    >
                      <Icon name="close" size={14} />
                    </Button>
                  </div>
                  <div className="routing-form-grid routing-rule-set-grid">
                    <div className="flex flex-col gap-1">
                      <FormInput label="Tag" value={ruleSetForm.tag} onChange={(v) => setRuleSetForm({ tag: v })} placeholder="geoip-cn" mono />
                    </div>
                    <div className="flex flex-col gap-1">
                      <FormSelect
                        label="Rule set type"
                        value={ruleSetForm.type}
                        onChange={(v) => setRuleSetForm({ type: v as 'remote' | 'local' })}
                        options={[{ value: 'remote', label: 'Remote' }, { value: 'local', label: 'Local' }]}
                      />
                    </div>
                    <div className="flex flex-col gap-1">
                      <FormSelect
                        label="Format"
                        value={ruleSetForm.format}
                        onChange={(v) => setRuleSetForm({ format: v as 'binary' | 'source' })}
                        options={[
                          { value: 'binary', label: 'Binary (.srs)' },
                          { value: 'source', label: 'Source (.json)' },
                        ]}
                      />
                    </div>
                    <div className="flex flex-col gap-1">
                      <FormSelect
                        label="Update interval"
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
                      className="font-semibold"
                      onClick={saveRuleSet}
                    >
                      Add
                    </Button>
                  </div>
                  {ruleSetForm.type === 'remote' && (
                    <div className="flex flex-col gap-1 mt-2.5">
                      <FormInput label="Remote URL" value={ruleSetForm.url || ''} onChange={(v) => setRuleSetForm({ url: v })} placeholder="https://cdn.jsdelivr.net/.../geoip/cn.srs" />
                    </div>
                  )}
                </div>
              )}

              <div className="routing-rule-set-list flex flex-col gap-1.5 overflow-y-auto pr-1">
                {ruleSets.map((rs) => (
                  <div key={rs.id} className="routing-rule-set-item flex items-center gap-2 p-2 bg-muted/40 rounded-lg min-w-0">
                    <Icon name="database" size={12} className="text-muted-foreground shrink-0" />
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
                          aria-label={`Update rule set ${rs.tag}`}
                          disabled={updatingRuleSet === rs.id}
                        >
                          <Icon name="download" size={12} className={updatingRuleSet === rs.id ? 'animate-spin' : ''} />
                        </Button>
                      )}
                      <Button
                        variant="ghost"
                        size="icon"
                        className="size-6 text-muted-foreground hover:text-destructive hover:bg-destructive/10 rounded"
                        onClick={() => deleteRuleSet(rs.id)}
                        aria-label={`Delete rule set ${rs.tag}`}
                      >
                        <Icon name="delete" size={12} />
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
        <Card className="routing-rules-panel p-4 bg-card border-border shadow-sm flex flex-col min-w-0">
          <CardHeader className="p-0 pb-3 flex flex-row items-center gap-2 space-y-0 shrink-0">
            <Icon name="filter_list" size={16} className="text-muted-foreground" />
            <CardTitle className="text-xs font-bold text-foreground">Routing Rules</CardTitle>
            <Badge variant="outline" className="h-4 px-1.5 text-[9px] border-border bg-muted/30">
              {routingRules.filter(r => r.enabled !== false).length} / {routingRules.length} active
            </Badge>
          </CardHeader>

          <CardContent className="p-0 flex-1 flex flex-col min-w-0">
            {showAddRule && (
              <div className="p-3 mb-3.5 bg-muted/40 rounded-lg shrink-0">
                <div className="routing-form-grid routing-rule-grid">
                  <div className="flex flex-col gap-1">
                    <FormSelect
                      label="Rule type"
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
                  <div className="flex flex-col gap-1">
                    {ruleForm.type === 'rule_set' ? (
                      <FormSelect
                        label="Rule value"
                        value={ruleForm.value}
                        onChange={(v) => setRuleForm({ value: v })}
                        options={[
                          { value: '', label: 'Select Set...' },
                          ...ruleSets.map((rs) => ({ value: rs.tag, label: rs.tag })),
                        ]}
                      />
                    ) : (
                      <FormInput
                        label="Rule value"
                        value={ruleForm.value}
                        onChange={(v) => setRuleForm({ value: v })}
                        placeholder={ruleForm.type === 'geoip' ? 'cn, private…' : ruleForm.type === 'ip_cidr' ? '10.0.0.0/8' : 'e.g. google.com'}
                        mono
                      />
                    )}
                  </div>
                  <div className="flex flex-col gap-1">
                    <FormSelect
                      label="Outbound"
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
                    <span className="text-xs font-medium text-muted-foreground">Invert</span>
                    <div className="h-8 flex items-center">
                      <Switch checked={ruleForm.invert} onCheckedChange={(v) => setRuleForm({ invert: v })} aria-label="Invert rule" />
                    </div>
                  </div>
                  <div className="flex flex-col gap-1">
                    <FormInput label="Notes" value={ruleForm.notes || ''} onChange={(v) => setRuleForm({ notes: v })} placeholder="e.g. China bypass" />
                  </div>
                  <div className="routing-form-actions flex gap-1 justify-end">
                    <Button
                      variant="default"
                      size="sm"
                      className="font-semibold"
                      onClick={saveRoutingRule}
                    >
                      {editingRule ? 'Update' : 'Add'}
                    </Button>
                    <Button
                      variant="outline"
                      size="icon"
                      className="size-8"
                      onClick={() => { setShowAddRule(false); setEditingRule(null); }}
                      aria-label="Cancel editing rule"
                    >
                      <Icon name="close" size={16} />
                    </Button>
                  </div>
                </div>
              </div>
            )}

            <div className="routing-table-scroll">
              <div className="rules-table rounded-lg bg-muted/10">
                <div className="rules-table-header bg-muted/40 border-b border-border/60 text-[10px] uppercase font-bold tracking-wider text-muted-foreground py-2.5 px-3 flex items-center shrink-0">
                  <span className="w-7">#</span>
                  <span className="w-16">Active</span>
                  <span className="w-32 shrink-0">Type</span>
                  <span className="flex-1 truncate">Value</span>
                  <span className="w-24 shrink-0">Outbound</span>
                  <span className="flex-1 truncate">Notes</span>
                  <span className="w-16 shrink-0 text-right">Actions</span>
                </div>
                <div className="flex flex-col divide-y divide-border/30">
                  {routingRules.map((rule, i) => (
                    <div
                      key={rule.id}
                      className={`flex items-center text-xs py-2 px-3 hover:bg-muted/20 ${
                        rule.enabled === false ? 'opacity-40' : ''
                      }`}
                    >
                      <span className="w-7 text-[10px] text-muted-foreground font-mono">{i + 1}</span>
                      <span className="w-16 flex items-center">
                        <Switch
                          checked={rule.enabled !== false}
                          aria-label={`Enable rule ${rule.value}`}
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
                          aria-label={`Edit rule ${rule.value}`}
                        >
                          <Icon name="edit" size={12} />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="size-6 text-muted-foreground hover:text-destructive hover:bg-destructive/10 rounded"
                          onClick={() => deleteRoutingRule(rule.id)}
                          aria-label={`Delete rule ${rule.value}`}
                        >
                          <Icon name="delete" size={12} />
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
              <Icon name="info" size={14} className="mt-0.5 text-foreground shrink-0" />
              <span>Rules evaluate top-to-bottom. First match wins. Hardcoded: dns → dns-out, private IPs → direct.</span>
            </div>
          </CardContent>
        </Card>
      </div>
    </ViewShell>
  );
}
