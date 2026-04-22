import { lazy, Suspense } from 'react';
import { createBrowserRouter, Navigate, Outlet } from 'react-router-dom';
import { AdminGuard, MemberGuard } from './guards';
import { PageSkeleton } from '@/components/shared/PageSkeleton';
import { AuthProvider } from '@/components/shared/AuthProvider';

// Layouts
const AdminLayout = lazy(() => import('@/components/admin/AdminLayout'));
const PublicLayout = lazy(() => import('@/components/public/PublicLayout'));

// Admin pages
const AdminLoginPage = lazy(() => import('@/pages/admin/LoginPage'));
const DashboardPage = lazy(() => import('@/pages/admin/DashboardPage'));
const ContentTypesPage = lazy(() => import('@/pages/admin/ContentTypesPage'));
const ContentPage = lazy(() => import('@/pages/admin/ContentPage'));
const MediaPage = lazy(() => import('@/pages/admin/MediaPage'));
const PluginsPage = lazy(() => import('@/pages/admin/PluginsPage'));
const UsersPage = lazy(() => import('@/pages/admin/UsersPage'));
const RolesPage = lazy(() => import('@/pages/admin/RolesPage'));
const SettingsPage = lazy(() => import('@/pages/admin/SettingsPage'));
const PluginNamespacePage = lazy(() => import('@/pages/admin/PluginNamespacePage'));

// Public pages
const HomePage = lazy(() => import('@/pages/public/HomePage'));
const ContentIndexPage = lazy(() => import('@/pages/public/ContentIndexPage'));
const ContentListPage = lazy(() => import('@/pages/public/ContentListPage'));
const ContentDetailPage = lazy(() => import('@/pages/public/ContentDetailPage'));
const SearchPage = lazy(() => import('@/pages/public/SearchPage'));
const NotFoundPage = lazy(() => import('@/pages/public/NotFoundPage'));

// Member pages
const MemberLoginPage = lazy(() => import('@/pages/member/LoginPage'));
const MemberRegisterPage = lazy(() => import('@/pages/member/RegisterPage'));
const ProfilePage = lazy(() => import('@/pages/member/ProfilePage'));

function Lazy({
  children,
  variant = 'list',
}: {
  children: React.ReactNode;
  variant?: 'list' | 'detail' | 'dashboard';
}) {
  return <Suspense fallback={<PageSkeleton variant={variant} />}>{children}</Suspense>;
}

export const router = createBrowserRouter([
  {
    element: (
      <AuthProvider>
        <Outlet />
      </AuthProvider>
    ),
    children: [
      // Admin login (no guard)
      {
        path: '/admin/login',
        element: <Lazy><AdminLoginPage /></Lazy>,
      },

      // Admin (guarded)
      {
        path: '/admin',
        element: <Lazy><AdminGuard /></Lazy>,
        children: [
          {
            element: <Lazy><AdminLayout /></Lazy>,
            children: [
              { index: true, element: <Navigate to="dashboard" replace /> },
              { path: 'dashboard', element: <Lazy><DashboardPage /></Lazy> },
              { path: 'content-types', element: <Lazy><ContentTypesPage /></Lazy> },
              { path: 'content', element: <Lazy><ContentPage /></Lazy> },
              { path: 'media', element: <Lazy><MediaPage /></Lazy> },
              { path: 'plugins', element: <Lazy><PluginsPage /></Lazy> },
              { path: 'users', element: <Lazy><UsersPage /></Lazy> },
              { path: 'roles', element: <Lazy><RolesPage /></Lazy> },
              { path: 'settings', element: <Lazy><SettingsPage /></Lazy> },
              { path: 'x/:plugin/*', element: <Lazy><PluginNamespacePage /></Lazy> },
            ],
          },
        ],
      },

      // Public + member
      {
        element: <Lazy><PublicLayout /></Lazy>,
        children: [
          { path: '/', element: <Lazy><HomePage /></Lazy> },
          { path: '/blog', element: <Lazy><ContentListPage /></Lazy> },
          { path: '/blog/:idOrSlug', element: <Lazy><ContentDetailPage /></Lazy> },
          { path: '/content', element: <Lazy><ContentIndexPage /></Lazy> },
          { path: '/content/:typeApiId', element: <Lazy><ContentListPage /></Lazy> },
          { path: '/content/:typeApiId/:idOrSlug', element: <Lazy><ContentDetailPage /></Lazy> },
          { path: '/search', element: <Lazy><SearchPage /></Lazy> },
          { path: '/login', element: <Lazy><MemberLoginPage /></Lazy> },
          { path: '/register', element: <Lazy><MemberRegisterPage /></Lazy> },

          // Member guarded
          {
            element: <Lazy><MemberGuard /></Lazy>,
            children: [
              { path: '/profile', element: <Lazy><ProfilePage /></Lazy> },
            ],
          },

          { path: '*', element: <Lazy><NotFoundPage /></Lazy> },
        ],
      },
    ],
  },
]);
