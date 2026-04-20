import { createContext, useContext } from 'react';
import type { User } from '@/types';

export interface AuthState {
  user: User | null;
  loading: boolean;
  isAdmin: boolean;
  isMember: boolean;
}

export interface AuthContextValue extends AuthState {
  setUser: (user: User | null) => void;
  refresh: () => Promise<void>;
  logout: () => void;
}

export const AuthContext = createContext<AuthContextValue | null>(null);

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error('useAuth must be used within AuthProvider');
  return ctx;
}
