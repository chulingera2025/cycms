import { useMemo, useState } from 'react';
import { Link, Outlet, useLocation, useNavigate } from 'react-router-dom';
import {
  Alert,
  Avatar,
  Breadcrumb,
  Button,
  Drawer,
  Dropdown,
  Grid,
  Layout,
  Menu,
  type MenuProps,
} from 'antd';
import {
  Database,
  FileText,
  Image as ImageIcon,
  LayoutDashboard,
  LogOut,
  Menu as MenuIcon,
  PanelLeftClose,
  PanelLeftOpen,
  Puzzle,
  Settings,
  Shield,
  UserCircle,
  Users,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import {
  AdminExtensionRegistryProvider,
  useAdminExtensions,
} from '@/features/admin-extensions';
import { useAuth } from '@/stores/auth';
import { ThemeSwitcher } from '@/components/shared/ThemeSwitcher';

const { Sider, Header, Content } = Layout;

interface NavEntry {
  key: string;
  label: string;
  icon: React.ReactNode;
  zone: string;
  order: number;
}

const ZONE_ORDER: Record<string, number> = {
  content: 10,
  plugins: 20,
  system: 30,
  settings: 40,
};

const ROUTE_LABELS: Record<string, string> = {
  '/admin/write': '写文章',
  '/admin/pages': '管理页面',
  '/admin/site-settings': '站点设置',
};

function resolvePluginIcon(icon?: string) {
  switch (icon) {
    case 'database':
      return <Database size={16} />;
    case 'media':
    case 'image':
      return <ImageIcon size={16} />;
    case 'shield':
      return <Shield size={16} />;
    case 'settings':
      return <Settings size={16} />;
    case 'users':
      return <Users size={16} />;
    case 'file-text':
    case 'book-open':
      return <FileText size={16} />;
    default:
      return <Puzzle size={16} />;
  }
}

function AdminLayoutContent() {
  const { user, logout } = useAuth();
  const {
    degraded,
    error,
    dismissRevisionChange,
    findRoute,
    findSettingsPage,
    menuItems: pluginMenus,
    revisionChange,
  } = useAdminExtensions();
  const location = useLocation();
  const navigate = useNavigate();
  const { t } = useTranslation(['admin', 'common']);
  const screens = Grid.useBreakpoint();
  const isMobile = !screens.lg;
  const [collapsed, setCollapsed] = useState(false);
  const [drawerOpen, setDrawerOpen] = useState(false);

  const coreNavEntries = useMemo<NavEntry[]>(
    () => [
      { key: '/admin/dashboard', label: t('nav.dashboard'), icon: <LayoutDashboard size={16} />, zone: 'content', order: 0 },
      { key: '/admin/content-types', label: t('nav.contentTypes'), icon: <Database size={16} />, zone: 'content', order: 10 },
      { key: '/admin/content', label: t('nav.content'), icon: <FileText size={16} />, zone: 'content', order: 20 },
      { key: '/admin/media', label: t('nav.media'), icon: <ImageIcon size={16} />, zone: 'content', order: 30 },
      { key: '/admin/plugins', label: t('nav.plugins'), icon: <Puzzle size={16} />, zone: 'plugins', order: 0 },
      { key: '/admin/users', label: t('nav.users'), icon: <Users size={16} />, zone: 'system', order: 0 },
      { key: '/admin/roles', label: t('nav.roles'), icon: <Shield size={16} />, zone: 'system', order: 10 },
      { key: '/admin/settings', label: t('nav.settings'), icon: <Settings size={16} />, zone: 'settings', order: 0 },
    ],
    [t],
  );

  const navEntries = useMemo<NavEntry[]>(
    () =>
      [...coreNavEntries, ...pluginMenus.map((menu) => ({
        key: menu.fullPath,
        label: menu.label,
        icon: resolvePluginIcon(menu.icon),
        zone: menu.zone,
        order: menu.order,
      }))].sort((left, right) => {
        const zoneCompare = (ZONE_ORDER[left.zone] ?? 100) - (ZONE_ORDER[right.zone] ?? 100);
        if (zoneCompare !== 0) {
          return zoneCompare;
        }
        if (left.order !== right.order) {
          return left.order - right.order;
        }
        return left.label.localeCompare(right.label, 'zh-CN');
      }),
    [coreNavEntries, pluginMenus],
  );

  const menuItems: MenuProps['items'] = navEntries.map((n) => ({
    key: n.key,
    icon: n.icon,
    label: n.label,
  }));

  const selectedEntry = navEntries.find((entry) => {
    if (location.pathname.startsWith('/admin/write') || location.pathname.startsWith('/admin/pages')) {
      return entry.key === '/admin/content';
    }
    if (location.pathname.startsWith('/admin/site-settings')) {
      return entry.key === '/admin/settings';
    }
    if (entry.key === '/admin/dashboard') {
      return location.pathname === entry.key;
    }
    return location.pathname === entry.key || location.pathname.startsWith(`${entry.key}/`);
  });

  const namespaceLabel = useMemo(() => {
    if (!location.pathname.startsWith('/admin/x/')) {
      return null;
    }
    const segments = location.pathname.replace('/admin/x/', '').split('/');
    const pluginName = segments[0] ?? '';
    const tail = segments.length > 1 ? `/${segments.slice(1).join('/')}` : '/';
    const route = findRoute(pluginName, tail);
    if (route) {
      return route.title;
    }
    const settingsPage = findSettingsPage(pluginName, tail);
    return settingsPage ? `${pluginName} 设置` : null;
  }, [findRoute, findSettingsPage, location.pathname]);

  const staticRouteLabel = useMemo(
    () => Object.entries(ROUTE_LABELS).find(([prefix]) => location.pathname.startsWith(prefix))?.[1] ?? null,
    [location.pathname],
  );

  const currentLabel = selectedEntry?.label ?? staticRouteLabel ?? namespaceLabel ?? (location.pathname === '/admin' ? t('nav.dashboard') : '');

  function handleMenuClick(key: string) {
    navigate(key);
    if (isMobile) setDrawerOpen(false);
  }

  function handleLogout() {
    logout();
    navigate('/admin/login');
  }

  const userMenu: MenuProps['items'] = [
    {
      key: 'logout',
      icon: <LogOut size={14} />,
      label: t('common:actions.logout'),
      onClick: handleLogout,
    },
  ];

  const brand = (
    <Link
      to="/admin/dashboard"
      className="flex h-14 items-center gap-2 border-b border-white/5 px-4 text-white no-underline hover:text-white"
    >
      <span className="grid h-7 w-7 flex-none place-items-center rounded bg-brand text-sm font-bold text-white">
        C
      </span>
      {!collapsed && (
        <span className="text-base font-semibold tracking-wide text-white">
          {t('app.name', { ns: 'common' })}
        </span>
      )}
    </Link>
  );

  const SidebarContent = (
    <>
      {brand}
      <Menu
        theme="dark"
        mode="inline"
        selectedKeys={selectedEntry ? [selectedEntry.key] : []}
        items={menuItems}
        onClick={({ key }) => handleMenuClick(key as string)}
        style={{ borderInlineEnd: 0 }}
      />
    </>
  );

  return (
    <Layout className="min-h-screen">
      {!isMobile && (
        <Sider
          width={240}
          collapsedWidth={80}
          collapsed={collapsed}
          trigger={null}
          collapsible
          style={{ overflow: 'hidden', position: 'sticky', top: 0, height: '100vh' }}
        >
          {SidebarContent}
        </Sider>
      )}
      {isMobile && (
        <Drawer
          open={drawerOpen}
          onClose={() => setDrawerOpen(false)}
          placement="left"
          width={240}
          closable={false}
          styles={{ body: { padding: 0, background: 'var(--color-sidebar-bg)' } }}
        >
          {SidebarContent}
        </Drawer>
      )}
      <Layout>
        <Header className="sticky top-0 z-20 flex items-center justify-between gap-4 border-b border-border px-4">
          <div className="flex items-center gap-3">
            <Button
              type="text"
              aria-label={t('actions.refresh', { ns: 'common' })}
              icon={
                isMobile ? (
                  <MenuIcon size={18} />
                ) : collapsed ? (
                  <PanelLeftOpen size={18} />
                ) : (
                  <PanelLeftClose size={18} />
                )
              }
              onClick={() =>
                isMobile ? setDrawerOpen(true) : setCollapsed((v) => !v)
              }
            />
            <Breadcrumb
              items={[
                { title: t('app.name', { ns: 'common' }) },
                { title: currentLabel },
              ]}
            />
          </div>
          <div className="flex items-center gap-1">
            <ThemeSwitcher />
            <Dropdown menu={{ items: userMenu }} placement="bottomRight" trigger={['click']}>
              <button
                type="button"
                className="inline-flex items-center gap-2 rounded px-2 py-1 text-text-secondary transition-colors hover:bg-surface-alt hover:text-text"
              >
                <Avatar size={24} icon={<UserCircle size={16} />} />
                <span className="hidden text-sm md:inline">{user?.username ?? '-'}</span>
              </button>
            </Dropdown>
          </div>
        </Header>
        <Content>
          {degraded && (
            <div className="p-4 pb-0">
              <Alert
                type="warning"
                showIcon
                message="插件扩展注册表加载失败，后台已切换到降级模式"
                description={error?.message ?? '核心后台页面仍可继续使用，插件菜单和命名空间路由将回退到最近一次成功加载的注册表。'}
              />
            </div>
          )}
          {revisionChange && (
            <div className="p-4 pb-0">
              <Alert
                type="info"
                showIcon
                closable
                onClose={dismissRevisionChange}
                message="插件扩展注册表已更新"
                description={`检测到插件扩展注册表已从 ${revisionChange.previousRevision} 更新为 ${revisionChange.currentRevision}。当前后台菜单、命名空间路由和设置入口已经按最新状态重算。`}
              />
            </div>
          )}
          <Outlet />
        </Content>
      </Layout>
    </Layout>
  );
}

export default function AdminLayout() {
  return (
    <AdminExtensionRegistryProvider>
      <AdminLayoutContent />
    </AdminExtensionRegistryProvider>
  );
}
