import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import PluginsPage from './PluginsPage';
import { adminExtensionsApi } from '@/lib/api/admin-extensions';
import { usePluginAction, usePlugins } from '@/features/plugins/hooks';
import type { AdminExtensionDiagnostics, Plugin } from '@/types';

vi.mock('@/features/plugins/hooks', () => ({
  usePlugins: vi.fn(),
  usePluginAction: vi.fn(),
}));

vi.mock('@/lib/api/admin-extensions', () => ({
  adminExtensionsApi: {
    diagnostics: vi.fn(),
  },
}));

vi.mock('@/lib/toast', () => ({
  toast: {
    success: vi.fn(),
  },
}));

const usePluginsMock = vi.mocked(usePlugins);
const usePluginActionMock = vi.mocked(usePluginAction);
const diagnosticsMock = vi.mocked(adminExtensionsApi.diagnostics);

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <PluginsPage />
    </QueryClientProvider>,
  );
}

describe('PluginsPage', () => {
  beforeEach(() => {
    const plugins: Plugin[] = [
      {
        name: 'demo-plugin',
        version: '0.1.0',
        runtime: 'native',
        status: 'enabled',
        description: 'Demo plugin',
      },
    ];

    usePluginsMock.mockReturnValue({
      data: plugins,
      isLoading: false,
      refetch: vi.fn().mockResolvedValue(undefined),
      isRefetching: false,
    } as unknown as ReturnType<typeof usePlugins>);

    usePluginActionMock.mockReturnValue({
      mutateAsync: vi.fn().mockResolvedValue(undefined),
      isPending: false,
    } as unknown as ReturnType<typeof usePluginAction>);

    const diagnostics: AdminExtensionDiagnostics = {
      revision: 'rev-42',
      diagnostics: [
        {
          pluginName: 'demo-plugin',
          pluginVersion: '0.1.0',
          severity: 'warning',
          code: 'asset-missing',
          message: 'frontend manifest 缺少可选样式文件',
        },
      ],
      recentEvents: [
        {
          id: 'evt-1',
          source: 'host',
          level: 'error',
          eventName: 'module.mount.error',
          message: 'slot demo-plugin:sidebar 挂载失败',
          recordedAt: '2025-01-01T00:00:00Z',
          pluginName: 'demo-plugin',
          contributionId: 'sidebar',
          contributionKind: 'slot',
          fullPath: '/admin/content/posts/new',
          requestId: 'req-1',
          detail: { slotId: 'content.editor.sidebar' },
        },
      ],
      security: {
        cspEnabled: true,
        cspReportOnly: true,
        cspHeaderName: 'Content-Security-Policy-Report-Only',
        cspPolicy: "default-src 'self'; script-src 'self'",
        cspReportUri: '/api/v1/admin/extensions/events',
      },
    };

    diagnosticsMock.mockResolvedValue(diagnostics);
  });

  it('renders diagnostics drawer with security state and recent events', async () => {
    renderPage();

    fireEvent.click(screen.getByRole('button', { name: '扩展诊断' }));

    await screen.findByText('扩展诊断与最近事件');
    expect(await screen.findByText('rev-42')).toBeInTheDocument();
    expect(await screen.findByText('Content-Security-Policy-Report-Only（report-only）')).toBeInTheDocument();
    expect(await screen.findByText('frontend manifest 缺少可选样式文件')).toBeInTheDocument();
    expect(await screen.findByText('slot demo-plugin:sidebar 挂载失败')).toBeInTheDocument();
    expect(await screen.findByText(/content.editor.sidebar/)).toBeInTheDocument();

    await waitFor(() => {
      expect(diagnosticsMock).toHaveBeenCalledTimes(1);
    });
  });
});