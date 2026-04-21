import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import AdminLayout from './AdminLayout';
import { useAdminExtensions } from '@/features/admin-extensions';
import { useAuth } from '@/stores/auth';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock('@/features/admin-extensions', () => ({
  AdminExtensionRegistryProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  useAdminExtensions: vi.fn(),
}));

vi.mock('@/stores/auth', () => ({
  useAuth: vi.fn(),
}));

vi.mock('@/components/shared/ThemeSwitcher', () => ({
  ThemeSwitcher: () => <div>theme-switcher</div>,
}));

const useAdminExtensionsMock = vi.mocked(useAdminExtensions);
const useAuthMock = vi.mocked(useAuth);

describe('AdminLayout', () => {
  it('renders plugin menus inside the admin shell navigation', () => {
    useAdminExtensionsMock.mockReturnValue({
      degraded: false,
      error: null,
      dismissRevisionChange: vi.fn(),
      findRoute: vi.fn(),
      findSettingsPage: vi.fn(),
      menuItems: [
        {
          pluginName: 'demo',
          pluginVersion: '0.1.0',
          id: 'menu.demo.reports',
          label: 'Plugin Reports',
          zone: 'plugins',
          icon: 'file-text',
          order: 5,
          to: '/reports',
          fullPath: '/admin/x/demo/reports',
        },
      ],
      revisionChange: null,
    } as unknown as ReturnType<typeof useAdminExtensions>);

    useAuthMock.mockReturnValue({
      user: { username: 'admin' },
      logout: vi.fn(),
    } as unknown as ReturnType<typeof useAuth>);

    render(
      <MemoryRouter initialEntries={['/admin/dashboard']}>
        <Routes>
          <Route path="/admin" element={<AdminLayout />}>
            <Route path="dashboard" element={<div>dashboard</div>} />
          </Route>
        </Routes>
      </MemoryRouter>,
    );

    fireEvent.click(screen.getByRole('button', { name: 'actions.refresh' }));

    expect(screen.getByText('Plugin Reports')).toBeInTheDocument();
    expect(screen.getByText('dashboard')).toBeInTheDocument();
  });
});