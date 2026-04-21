import { api } from './client';
import type {
  ContentEntry,
  CreateEntryInput,
  UpdateEntryInput,
  PaginatedResponse,
  RevisionListResponse,
  Revision,
} from '@/types';

const BASE = '/api/v1/content';

export const contentApi = {
  list(
    typeApiId: string,
    params?: Record<string, string>,
  ): Promise<PaginatedResponse<ContentEntry>> {
    return api.get<PaginatedResponse<ContentEntry>>(
      `${BASE}/${typeApiId}`,
      params,
    );
  },

  get(
    typeApiId: string,
    id: string,
    populate?: string[],
  ): Promise<ContentEntry> {
    const params = populate?.length ? { populate: populate.join(',') } : undefined;
    return api.get<ContentEntry>(`${BASE}/${typeApiId}/${id}`, params);
  },

  create(typeApiId: string, data: CreateEntryInput): Promise<ContentEntry> {
    return api.post<ContentEntry>(`${BASE}/${typeApiId}`, data);
  },

  update(
    typeApiId: string,
    id: string,
    data: UpdateEntryInput,
  ): Promise<ContentEntry> {
    return api.put<ContentEntry>(`${BASE}/${typeApiId}/${id}`, data);
  },

  delete(typeApiId: string, id: string, mode?: 'soft' | 'hard'): Promise<void> {
    const qs = mode ? `?mode=${mode}` : '';
    return api.delete<void>(`${BASE}/${typeApiId}/${id}${qs}`);
  },

  publish(typeApiId: string, id: string): Promise<ContentEntry> {
    return api.post<ContentEntry>(`${BASE}/${typeApiId}/${id}/publish`);
  },

  unpublish(typeApiId: string, id: string): Promise<ContentEntry> {
    return api.post<ContentEntry>(`${BASE}/${typeApiId}/${id}/unpublish`);
  },

  listRevisions(
    typeApiId: string,
    id: string,
    params?: Record<string, string>,
  ): Promise<RevisionListResponse> {
    return api.get<RevisionListResponse>(
      `${BASE}/${typeApiId}/${id}/revisions`,
      params,
    );
  },

  getRevision(
    typeApiId: string,
    id: string,
    version: number,
  ): Promise<Revision> {
    return api.get<Revision>(
      `${BASE}/${typeApiId}/${id}/revisions/${version}`,
    );
  },

  rollbackRevision(
    typeApiId: string,
    id: string,
    version: number,
  ): Promise<ContentEntry> {
    return api.post<ContentEntry>(
      `${BASE}/${typeApiId}/${id}/revisions/${version}/rollback`,
    );
  },
};
