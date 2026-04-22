import { type ReactNode, useCallback, useMemo, useState } from 'react';
import { QueryClientProvider } from '@tanstack/react-query';
import { App as AntApp, ConfigProvider } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import { BrowserRouter } from 'react-router-dom';
import { AdminExtensionRegistryProvider } from '@/features/admin-extensions';
import type { HostIslandMountContext } from '@/features/islands/runtime';
import { queryClient } from '@/lib/query-client';
import { ThemeProvider, useTheme } from '@/lib/theme-provider';
import { useBindAntdApi } from '@/lib/toast';
import { darkTheme, lightTheme } from '@/styles/theme';
import { ErrorBoundary } from '@/components/shared/ErrorBoundary';
import { AuthContext } from '@/stores/auth';
import type { User } from '@/types';

const ADMIN_ROLES = ['super_admin', 'editor', 'author'];

interface HostIslandShellProps {
  auth: HostIslandMountContext['auth'];
  children: ReactNode;
}

function deriveFlags(user: User | null) {
  const roles = user?.roles ?? [];
  return {
    isAdmin: roles.some((role) => ADMIN_ROLES.includes(role)),
    isMember: user !== null && !roles.some((role) => ADMIN_ROLES.includes(role)),
  };
}

function AntdApiBinder({ children }: { children: ReactNode }) {
  useBindAntdApi();
  return <>{children}</>;
}

function HostIslandAuthProvider({
  auth,
  children,
}: {
  auth: HostIslandMountContext['auth'];
  children: ReactNode;
}) {
  const [user, setUserState] = useState<User | null>(auth.user);
  const flags = useMemo(() => deriveFlags(user), [user]);

  const setUser = useCallback((nextUser: User | null) => {
    setUserState(nextUser);
  }, []);

  const refresh = useCallback(async () => {
    await auth.refresh();
    setUserState(auth.user);
  }, [auth]);

  const logout = useCallback(() => {
    auth.logout();
    setUserState(null);
  }, [auth]);

  const value = useMemo(
    () => ({
      user,
      loading: false,
      isAdmin: flags.isAdmin,
      isMember: flags.isMember,
      setUser,
      refresh,
      logout,
    }),
    [flags.isAdmin, flags.isMember, logout, refresh, setUser, user],
  );

  return <AuthContext value={value}>{children}</AuthContext>;
}

function ThemedHostIslandShell({ auth, children }: HostIslandShellProps) {
  const { resolved } = useTheme();

  return (
    <ConfigProvider theme={resolved === 'dark' ? darkTheme : lightTheme} locale={zhCN}>
      <AntApp>
        <AntdApiBinder>
          <ErrorBoundary>
            <BrowserRouter>
              <HostIslandAuthProvider auth={auth}>
                <AdminExtensionRegistryProvider>{children}</AdminExtensionRegistryProvider>
              </HostIslandAuthProvider>
            </BrowserRouter>
          </ErrorBoundary>
        </AntdApiBinder>
      </AntApp>
    </ConfigProvider>
  );
}

export function HostIslandShell({ auth, children }: HostIslandShellProps) {
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <ThemedHostIslandShell auth={auth}>{children}</ThemedHostIslandShell>
      </ThemeProvider>
    </QueryClientProvider>
  );
}