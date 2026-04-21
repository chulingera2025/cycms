import { api, setTokens, clearTokens } from './client';
import type { TokenPair, User, LoginRequest, RegisterRequest } from '@/types';

export const authApi = {
  login(data: LoginRequest): Promise<TokenPair> {
    return api.post<TokenPair>('/api/v1/auth/login', data).then((pair) => {
      setTokens(pair.access_token, pair.refresh_token);
      return pair;
    });
  },

  register(data: RegisterRequest): Promise<User> {
    return api.post<User>('/api/v1/auth/register', data);
  },

  me(): Promise<User> {
    return api.get<User>('/api/v1/auth/me');
  },

  logout() {
    clearTokens();
  },
};
