import { create } from 'zustand';
import { storeHelper, type RoutingRule, type RuleSet } from '../utils/store';
import { useLogStore } from './logStore';
import { invoke } from '@tauri-apps/api/core';

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
  saveRuleSet: () => Promise<void>;
  deleteRuleSet: (id: string) => Promise<void>;
  updateRuleSet: (rs: RuleSet) => Promise<void>;
}

export const useRoutingStore = create<RoutingState>((set, get) => ({
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

    set({ routingRules: updated, showAddRule: false, editingRule: null });
    await storeHelper.saveRoutingRules(updated);
    set({ ruleForm: { type: 'geoip', value: '', outbound: 'direct', invert: false, notes: '' } });
  },

  deleteRoutingRule: async (id) => {
    const updated = get().routingRules.filter((r) => r.id !== id);
    set({ routingRules: updated });
    await storeHelper.saveRoutingRules(updated);
  },

  saveRuleSet: async () => {
    const { ruleSetForm, ruleSets } = get();
    if (!ruleSetForm.tag.trim()) return;

    const updated = [...ruleSets, { ...ruleSetForm, id: uid() }];
    set({ ruleSets: updated, showAddRuleSet: false });
    await storeHelper.saveRuleSets(updated);
    set({ ruleSetForm: { tag: '', type: 'remote', format: 'binary', url: '', updateInterval: '1d' } });
  },

  deleteRuleSet: async (id) => {
    const updated = get().ruleSets.filter((r) => r.id !== id);
    set({ ruleSets: updated });
    await storeHelper.saveRuleSets(updated);
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
}));
