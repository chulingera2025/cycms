import { api, setTokens } from './client';
import type {
  ContentEntry,
  PaginatedResponse,
  PublicContentType,
  TokenPair,
  LoginRequest,
  RegisterRequest,
  User,
} from '@/types';

const BASE = '/api/v1/public';

export const publicApi = {
  listContentTypes(): Promise<PublicContentType[]> {
    return api.get<PublicContentType[]>(`${BASE}/content-types`);
  },

  listContent(
    typeApiId: string,
    params?: Record<string, string>,
  ): Promise<PaginatedResponse<ContentEntry>> {
    return api.get<PaginatedResponse<ContentEntry>>(
      `${BASE}/content/${typeApiId}`,
      params,
    );
  },

  getContent(
    typeApiId: string,
    idOrSlug: string,
    populate?: string[],
  ): Promise<ContentEntry> {
    const params = populate?.length ? { populate: populate.join(',') } : undefined;
    return api.get<ContentEntry>(
      `${BASE}/content/${typeApiId}/${idOrSlug}`,
      params,
    );
  },

  login(data: LoginRequest): Promise<TokenPair> {
    return api.post<TokenPair>(`${BASE}/auth/login`, data).then((pair) => {
      setTokens(pair.access_token, pair.refresh_token);
      return pair;
    });
  },

  register(data: RegisterRequest): Promise<User> {
    return api.post<User>(`${BASE}/auth/register`, data);
  },

  refresh(refreshToken: string): Promise<TokenPair> {
    return api.post<TokenPair>(`${BASE}/auth/refresh`, {
      refresh_token: refreshToken,
    });
  },
};
