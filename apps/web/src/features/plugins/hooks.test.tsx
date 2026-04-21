import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { usePluginAction } from './hooks';
import { invalidateAdminExtensionQueries } from '@/features/admin-extensions/invalidation';
import { reportAdminExtensionEvent } from '@/features/admin-extensions/telemetry';
import { pluginsApi } from '@/lib/api';

vi.mock('@/features/admin-extensions/invalidation', () => ({
  invalidateAdminExtensionQueries: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@/features/admin-extensions/telemetry', () => ({
  reportAdminExtensionEvent: vi.fn(),
}));

vi.mock('@/lib/api', () => ({
  pluginsApi: {
    list: vi.fn(),
    install: vi.fn().mockResolvedValue(undefined),
    enable: vi.fn().mockResolvedValue(undefined),
    disable: vi.fn().mockResolvedValue(undefined),
    uninstall: vi.fn().mockResolvedValue(undefined),
  },
}));

const invalidateMock = vi.mocked(invalidateAdminExtensionQueries);
const reportEventMock = vi.mocked(reportAdminExtensionEvent);

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
  };
}

describe('usePluginAction', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it.each([
    ['install', 'install'],
    ['enable', 'enable'],
    ['disable', 'disable'],
    ['uninstall', 'uninstall'],
  ] as const)('executes %s lifecycle action and reports success', async (action, apiMethod) => {
    const wrapper = createWrapper();
    const { result } = renderHook(() => usePluginAction(action), { wrapper });

    await act(async () => {
      await result.current.mutateAsync('demo-plugin');
    });

    expect(vi.mocked(pluginsApi[apiMethod])).toHaveBeenCalledWith('demo-plugin');
    expect(invalidateMock).toHaveBeenCalledTimes(1);
    expect(reportEventMock).toHaveBeenCalledWith(
      expect.objectContaining({
        eventName: `plugin.${action}.success`,
        pluginName: 'demo-plugin',
      }),
    );
  });

  it('reports structured error events when plugin action fails', async () => {
    vi.mocked(pluginsApi.uninstall).mockRejectedValueOnce(new Error('permission denied'));

    const wrapper = createWrapper();
    const { result } = renderHook(() => usePluginAction('uninstall'), { wrapper });

    await expect(
      act(async () => {
        await result.current.mutateAsync('demo-plugin');
      }),
    ).rejects.toThrow('permission denied');

    await waitFor(() => {
      expect(reportEventMock).toHaveBeenCalledWith(
        expect.objectContaining({
          eventName: 'plugin.uninstall.error',
          pluginName: 'demo-plugin',
          level: 'error',
        }),
      );
    });
  });
});