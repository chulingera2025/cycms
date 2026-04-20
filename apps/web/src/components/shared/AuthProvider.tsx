import { useState, useCallback, useEffect, type ReactNode } from 'react';
import { AuthContext, type AuthState } from '@/stores/auth';
import { authApi } from '@/api/auth';
import { getAccessToken, clearTokens } from '@/api/client';
import type { User } from '@/types';

const ADMIN_ROLES = ['super_admin', 'editor', 'author'];

function deriveFlags(user: User | null) {
  const roles = user?.roles ?? [];
  return {
    isAdmin: roles.some((r) => ADMIN_ROLES.includes(r)),
    isMember: user !== null && !roles.some((r) => ADMIN_ROLES.includes(r)),
  };
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<AuthState>({
    user: null,
    loading: true,
    isAdmin: false,
    isMember: false,
  });

  const setUser = useCallback((user: User | null) => {
    setState({ user, loading: false, ...deriveFlags(user) });
  }, []);

  const logout = useCallback(() => {
    clearTokens();
    setState({ user: null, loading: false, isAdmin: false, isMember: false });
  }, []);

  const refresh = useCallback(async () => {
    if (!getAccessToken()) {
      setState({ user: null, loading: false, isAdmin: false, isMember: false });
      return;
    }
    try {
      const user = await authApi.me();
      setState({ user, loading: false, ...deriveFlags(user) });
    } catch {
      clearTokens();
      setState({ user: null, loading: false, isAdmin: false, isMember: false });
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return (
    <AuthContext value={{ ...state, setUser, refresh, logout }}>
      {children}
    </AuthContext>
  );
}
