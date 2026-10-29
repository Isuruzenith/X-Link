import { Store } from '@tauri-apps/plugin-store';

type AppStore = Pick<Store, 'get' | 'set' | 'delete' | 'entries' | 'reload' | 'save'>;

const browserStores = new Map<string, Map<string, unknown>>();

export function isTauriEnvironment(): boolean {
  return typeof window !== 'undefined' && Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
}

export async function loadAppStore(path: string): Promise<AppStore> {
  if (isTauriEnvironment()) return Store.load(path);

  let entries = browserStores.get(path);
  if (!entries) {
    entries = new Map();
    browserStores.set(path, entries);
  }
  const data = entries;
  return {
    async get<T>(key: string) { return data.get(key) as T | undefined; },
    async set(key: string, value: unknown) { data.set(key, value); },
    async delete(key: string) { return data.delete(key); },
    async entries<T>() { return Array.from(data.entries()) as Array<[string, T]>; },
    async reload() {},
    async save() {},
  };
}
