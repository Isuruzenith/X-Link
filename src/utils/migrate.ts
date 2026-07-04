import { Store } from '@tauri-apps/plugin-store';
import type { RoutingRule } from './store';

export const CONFIG_SCHEMA_VERSION = 2;

async function getSettingsStore(): Promise<Store> {
  return Store.load('settings.json');
}

async function getRoutingStore(): Promise<Store> {
  return Store.load('routing.json');
}

async function migrateV1toV2(routingStore: Store): Promise<void> {
  // v1 had no 'enabled' field on RoutingRule — back-fill with true
  const rules = await routingStore.get<RoutingRule[]>('rules') ?? [];
  const migrated = rules.map((r) => ({ enabled: true, ...r }));
  await routingStore.set('rules', migrated);
  await routingStore.save();
}

export async function migrateStores(): Promise<void> {
  try {
    const store = await getSettingsStore();
    const v = await store.get<number>('_schemaVersion') ?? 1;

    if (v < 2) {
      const routingStore = await getRoutingStore();
      await migrateV1toV2(routingStore);
    }

    // Migrate default MTU from legacy 1500 to stable 1400
    const currentMtu = await store.get<number>('tunMtu');
    if (currentMtu === 1500 || currentMtu === null || currentMtu === undefined) {
      await store.set('tunMtu', 1400);
    }

    await store.set('_schemaVersion', CONFIG_SCHEMA_VERSION);
    await store.save();
  } catch (e) {
    // Migration must never block the app from starting
    console.error('[migrate] Schema migration failed (non-fatal):', e);
  }
}
