import { api } from './client';
import type { MediaAsset, MediaListResponse } from '@/types';

const BASE = '/api/v1/media';

export const mediaApi = {
  list(params?: Record<string, string>): Promise<MediaListResponse> {
    return api.get<MediaListResponse>(BASE, params);
  },

  get(id: string): Promise<MediaAsset> {
    return api.get<MediaAsset>(`${BASE}/${id}`);
  },

  upload(file: File, metadata?: Record<string, unknown>): Promise<MediaAsset> {
    const form = new FormData();
    form.append('file', file);
    if (metadata) {
      form.append('metadata', JSON.stringify(metadata));
    }
    return api.upload<MediaAsset>(`${BASE}/upload`, form);
  },

  delete(id: string): Promise<void> {
    return api.delete(`${BASE}/${id}`);
  },
};
