import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import PluginNamespacePage from './PluginNamespacePage';
import { useAdminExtensions } from '@/features/admin-extensions';
import { reportAdminExtensionEvent } from '@/features/admin-extensions/telemetry';

vi.mock('@/features/admin-extensions', () => ({
  useAdminExtensions: vi.fn(),
}));

vi.mock('@/features/admin-extensions/telemetry', () => ({
  reportAdminExtensionEvent: vi.fn(),
}));

vi.mock('@/features/admin-extensions/module-host', () => ({
  ModuleHostBoundary: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  PluginModuleHost: (props: { moduleUrl: string }) => <div>host:{props.moduleUrl}</div>,
}));

const useAdminExtensionsMock = vi.mocked(useAdminExtensions);
const reportEventMock = vi.mocked(reportAdminExtensionEvent);

function renderPage(initialEntry: string) {
  return render(
    <MemoryRouter initialEntries={[initialEntry]}>
      <Routes>
        <Route path="/admin/x/:plugin/*" element={<PluginNamespacePage />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe('PluginNamespacePage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('resolves namespace routes and emits success telemetry', async () => {
    useAdminExtensionsMock.mockReturnValue({
      bootstrap: { shellSdkVersion: '1.0.0' },
      degraded: false,
      dismissRevisionChange: vi.fn(),
      error: null,
      getPlugin: () => ({ name: 'demo', version: '0.1.0' }),
      findRoute: () => ({
        id: 'route.demo.reports',
        kind: 'page',
        fullPath: '/admin/x/demo/reports',
        moduleUrl: '/plugins/demo/reports.js',
        styles: ['/plugins/demo/reports.css'],
        title: 'Reports',
      }),
      findSettingsPage: () => null,
      isLoading: false,
      revision: 'rev-1',
      revisionChange: null,
    } as unknown as ReturnType<typeof useAdminExtensions>);

    renderPage('/admin/x/demo/reports');

    expect(screen.getByText('Reports')).toBeInTheDocument();
    expect(screen.getByText('host:/plugins/demo/reports.js')).toBeInTheDocument();

    await waitFor(() => {
      expect(reportEventMock).toHaveBeenCalledWith(
        expect.objectContaining({
          eventName: 'route.resolve.success',
          pluginName: 'demo',
          contributionId: 'route.demo.reports',
        }),
      );
    });
  });

  it('resolves custom settings pages through namespace routing', async () => {
    useAdminExtensionsMock.mockReturnValue({
      bootstrap: { shellSdkVersion: '1.0.0' },
      degraded: false,
      dismissRevisionChange: vi.fn(),
      error: null,
      getPlugin: () => ({ name: 'demo', version: '0.1.0' }),
      findRoute: () => null,
      findSettingsPage: () => ({
        plugin: { name: 'demo', version: '0.1.0' },
        settings: { namespace: 'demo', requiredPermissions: [], customPage: null },
        page: {
          path: '/settings',
          fullPath: '/admin/x/demo/settings',
          moduleUrl: '/plugins/demo/settings.js',
          styles: ['/plugins/demo/settings.css'],
        },
      }),
      isLoading: false,
      revision: 'rev-1',
      revisionChange: null,
    } as unknown as ReturnType<typeof useAdminExtensions>);

    renderPage('/admin/x/demo/settings');

    expect(screen.getByText('demo 设置')).toBeInTheDocument();
    expect(screen.getByText('host:/plugins/demo/settings.js')).toBeInTheDocument();
  });

  it('shows stale-route warning after revision invalidation and emits miss telemetry', async () => {
    useAdminExtensionsMock.mockReturnValue({
      bootstrap: { shellSdkVersion: '1.0.0' },
      degraded: false,
      dismissRevisionChange: vi.fn(),
      error: null,
      getPlugin: () => null,
      findRoute: () => null,
      findSettingsPage: () => null,
      isLoading: false,
      revision: 'rev-2',
      revisionChange: {
        previousRevision: 'rev-1',
        currentRevision: 'rev-2',
        changedAt: '2025-01-01T00:00:00Z',
      },
    } as unknown as ReturnType<typeof useAdminExtensions>);

    renderPage('/admin/x/demo/old');

    expect(screen.getByText('插件页面已失效')).toBeInTheDocument();

    await waitFor(() => {
      expect(reportEventMock).toHaveBeenCalledWith(
        expect.objectContaining({
          eventName: 'route.resolve.miss',
          pluginName: 'demo',
          level: 'warning',
        }),
      );
    });
  });
});