import { useMemo, useState } from 'react';
import { Link, Outlet, useLocation, useNavigate } from 'react-router-dom';
import {
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
import { useAuth } from '@/stores/auth';
import { ThemeSwitcher } from '@/components/shared/ThemeSwitcher';

const { Sider, Header, Content } = Layout;

interface NavEntry {
  key: string;
  labelKey: string;
  icon: React.ReactNode;
}

export default function AdminLayout() {
  const { user, logout } = useAuth();
  const location = useLocation();
  const navigate = useNavigate();
  const { t } = useTranslation(['admin', 'common']);
  const screens = Grid.useBreakpoint();
  const isMobile = !screens.lg;
  const [collapsed, setCollapsed] = useState(false);
  const [drawerOpen, setDrawerOpen] = useState(false);

  const navEntries = useMemo<NavEntry[]>(
    () => [
      { key: '/admin/dashboard', labelKey: 'nav.dashboard', icon: <LayoutDashboard size={16} /> },
      { key: '/admin/content-types', labelKey: 'nav.contentTypes', icon: <Database size={16} /> },
      { key: '/admin/content', labelKey: 'nav.content', icon: <FileText size={16} /> },
      { key: '/admin/media', labelKey: 'nav.media', icon: <ImageIcon size={16} /> },
      { key: '/admin/plugins', labelKey: 'nav.plugins', icon: <Puzzle size={16} /> },
      { key: '/admin/users', labelKey: 'nav.users', icon: <Users size={16} /> },
      { key: '/admin/roles', labelKey: 'nav.roles', icon: <Shield size={16} /> },
      { key: '/admin/settings', labelKey: 'nav.settings', icon: <Settings size={16} /> },
    ],
    [],
  );

  const menuItems: MenuProps['items'] = navEntries.map((n) => ({
    key: n.key,
    icon: n.icon,
    label: t(n.labelKey),
  }));

  const selectedKey =
    navEntries.find((n) => location.pathname.startsWith(n.key))?.key ?? '/admin/dashboard';
  const currentLabelKey = navEntries.find((n) => n.key === selectedKey)?.labelKey;

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
        selectedKeys={[selectedKey]}
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
                { title: currentLabelKey ? t(currentLabelKey) : '' },
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
          <Outlet />
        </Content>
      </Layout>
    </Layout>
  );
}
