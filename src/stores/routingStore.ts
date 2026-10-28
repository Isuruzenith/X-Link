import { create } from 'zustand';
import { storeHelper, type RoutingRule, type RuleSet } from '../utils/store';
import { useLogStore } from './logStore';
import { invoke } from '@tauri-apps/api/core';
import { connectedApplyDeps, createApplier, reportApplyOutcome } from '../utils/connectedApply';

interface RoutingSnapshot { rules: RoutingRule[]; ruleSets: RuleSet[] }

// Routing edits are persisted first, then applied to a connected session with a
// validated cold restart; on failure the snapshot taken before the first edit in
// the debounce window is restored (see utils/applyPolicy.ts).
let applierInstance: ReturnType<typeof createApplier<RoutingSnapshot>> | null = null;


const uid = () => Math.random().toString(36).slice(2, 10);

interface RoutingState {
  routingRules: RoutingRule[];
  ruleSets: RuleSet[];
  showAddRule: boolean;
  editingRule: RoutingRule | null;
  ruleForm: Omit<RoutingRule, 'id'>;
  showAddRuleSet: boolean;
  ruleSetForm: Omit<RuleSet, 'id'>;
  updatingRuleSet: string | null;

  // Actions
  initRouting: () => Promise<void>;
  setShowAddRule: (show: boolean) => void;
  setEditingRule: (rule: RoutingRule | null) => void;
  setRuleForm: (form: Partial<Omit<RoutingRule, 'id'>>) => void;
  setShowAddRuleSet: (show: boolean) => void;
  setRuleSetForm: (form: Partial<Omit<RuleSet, 'id'>>) => void;
  saveRoutingRule: () => Promise<void>;
  deleteRoutingRule: (id: string) => Promise<void>;
  toggleRoutingRuleEnabled: (id: string, enabled: boolean) => Promise<void>;
  saveRuleSet: () => Promise<void>;
  deleteRuleSet: (id: string) => Promise<void>;
  updateRuleSet: (rs: RuleSet) => Promise<void>;
}

export const useRoutingStore = create<RoutingState>((set, get) => {
  const getApplier = () => applierInstance ??= createApplier<RoutingSnapshot>({
    ...connectedApplyDeps,
    merge: (existing) => existing,
    validate: () => null,
    restore: async (baseline) => {
      set({ routingRules: baseline.rules, ruleSets: baseline.ruleSets });
      await storeHelper.saveRoutingRules(baseline.rules);
      await storeHelper.saveRuleSets(baseline.ruleSets);
    },
    report: reportApplyOutcome('Routing changes'),
  });
  const snapshot = (): RoutingSnapshot => ({ rules: get().routingRules, ruleSets: get().ruleSets });
  const scheduleApply = (baseline: RoutingSnapshot) => {
    if (connectedApplyDeps.isConnected()) getApplier().schedule(baseline);
  };

  return {
  routingRules: [],
  ruleSets: [],
  showAddRule: false,
  editingRule: null,
  ruleForm: { type: 'geoip', value: '', outbound: 'direct', invert: false, notes: '' },
  showAddRuleSet: false,
  ruleSetForm: { tag: '', type: 'remote', format: 'binary', url: '', updateInterval: '1d' },
  updatingRuleSet: null,

  initRouting: async () => {
    const rules = await storeHelper.getRoutingRules();
    const sets = await storeHelper.getRuleSets();
    set({ routingRules: rules, ruleSets: sets });
  },

  setShowAddRule: (show) => set({ showAddRule: show }),
  setEditingRule: (rule) => {
    set({ editingRule: rule });
    if (rule) {
      set({ ruleForm: { type: rule.type, value: rule.value, outbound: rule.outbound, invert: rule.invert, notes: rule.notes ?? '' } });
    }
  },
  setRuleForm: (form) => set((state) => ({ ruleForm: { ...state.ruleForm, ...form } })),

  setShowAddRuleSet: (show) => set({ showAddRuleSet: show }),
  setRuleSetForm: (form) => set((state) => ({ ruleSetForm: { ...state.ruleSetForm, ...form } })),

  saveRoutingRule: async () => {
    const { ruleForm, editingRule, routingRules } = get();
    if (!ruleForm.value.trim()) return;

    const updated = editingRule
      ? routingRules.map((r) => r.id === editingRule.id ? { ...ruleForm, id: editingRule.id } : r)
      : [...routingRules, { ...ruleForm, id: uid() }];

    const before = snapshot();
    set({ routingRules: updated, showAddRule: false, editingRule: null });
    await storeHelper.saveRoutingRules(updated);
    scheduleApply(before);
    set({ ruleForm: { type: 'geoip', value: '', outbound: 'direct', invert: false, notes: '' } });
  },

  deleteRoutingRule: async (id) => {
    const before = snapshot();
    const updated = get().routingRules.filter((r) => r.id !== id);
    set({ routingRules: updated });
    await storeHelper.saveRoutingRules(updated);
    scheduleApply(before);
  },

  toggleRoutingRuleEnabled: async (id, enabled) => {
    const before = snapshot();
    const updated = get().routingRules.map((r) =>
      r.id === id ? { ...r, enabled } : r
    );
    set({ routingRules: updated });
    await storeHelper.saveRoutingRules(updated);
    scheduleApply(before);
  },

  saveRuleSet: async () => {
    const { ruleSetForm, ruleSets } = get();
    if (!ruleSetForm.tag.trim()) return;

    const before = snapshot();
    const updated = [...ruleSets, { ...ruleSetForm, id: uid() }];
    set({ ruleSets: updated, showAddRuleSet: false });
    await storeHelper.saveRuleSets(updated);
    scheduleApply(before);
    set({ ruleSetForm: { tag: '', type: 'remote', format: 'binary', url: '', updateInterval: '1d' } });
  },

  deleteRuleSet: async (id) => {
    const before = snapshot();
    const updated = get().ruleSets.filter((r) => r.id !== id);
    set({ ruleSets: updated });
    await storeHelper.saveRuleSets(updated);
    scheduleApply(before);
  },

  updateRuleSet: async (rs) => {
    set({ updatingRuleSet: rs.id });
    const logStore = useLogStore.getState();
    try {
      await invoke('update_rule_set', { ruleSetId: rs.id, url: rs.url });
      const updated = get().ruleSets.map((r) => r.id === rs.id ? { ...r, lastUpdated: Date.now() } : r);
      set({ ruleSets: updated });
      await storeHelper.saveRuleSets(updated);
    } catch (err) {
      logStore.pushSystemLog(`Failed to update rule set: ${err}`);
    } finally {
      set({ updatingRuleSet: null });
    }
  }
  };
});
