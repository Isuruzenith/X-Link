import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { useNodeEditorStore } from './nodeEditorStore';
import { useProfileStore } from './profileStore';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

const native = vi.mocked(invoke);

beforeEach(() => {
  vi.clearAllMocks();
  useProfileStore.setState({
    profiles: [{ id: 'test', name: 'test', type: 'manual', importedAt: 1, lastUpdated: 1, nodeCount: 1, selectedNodeTag: 'Test' }],
    activeProfileId: 'test', selectedProfileId: 'test', selectedNodeTag: 'Test', nodes: [],
  });
  native.mockImplementation(async (command) => command === 'get_config_outbounds' ? [] : undefined);
});

describe('HTTPUpgrade node editing', () => {
  it('reads the transport host and saves it outside headers', async () => {
    useNodeEditorStore.getState().openEditor({
      type: 'vless', tag: 'Test', server: 'proxy.example', server_port: 443,
      uuid: '00000000-0000-0000-0000-000000000000',
      transport: { type: 'httpupgrade', path: '/azure', host: 'cdn.example' },
      tls: { enabled: true, server_name: 'tls.example', insecure: true },
    });
    expect(useNodeEditorStore.getState().editHost).toBe('cdn.example');

    await useNodeEditorStore.getState().saveEditor();

    expect(native).toHaveBeenCalledWith('update_node', {
      newOutbound: expect.objectContaining({
        transport: { type: 'httpupgrade', path: '/azure', host: 'cdn.example' },
      }),
    });
  });

  it('reads legacy HTTPUpgrade Host headers before they are normalized', () => {
    useNodeEditorStore.getState().openEditor({
      type: 'vless', tag: 'Test', server: 'proxy.example', server_port: 443,
      transport: { type: 'httpupgrade', headers: { Host: 'legacy.example' } },
    });
    expect(useNodeEditorStore.getState().editHost).toBe('legacy.example');
  });
});
