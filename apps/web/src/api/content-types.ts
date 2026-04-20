import { api } from './client';
import type {
  ContentTypeDefinition,
  CreateContentTypeInput,
  UpdateContentTypeInput,
} from '@/types';

const BASE = '/api/v1/content-types';

export const contentTypesApi = {
  list(): Promise<ContentTypeDefinition[]> {
    return api.get<ContentTypeDefinition[]>(BASE);
  },

  get(apiId: string): Promise<ContentTypeDefinition> {
    return api.get<ContentTypeDefinition>(`${BASE}/${apiId}`);
  },

  create(data: CreateContentTypeInput): Promise<ContentTypeDefinition> {
    return api.post<ContentTypeDefinition>(BASE, data);
  },

  update(apiId: string, data: UpdateContentTypeInput): Promise<ContentTypeDefinition> {
    return api.put<ContentTypeDefinition>(`${BASE}/${apiId}`, data);
  },

  delete(apiId: string): Promise<void> {
    return api.delete(`${BASE}/${apiId}`);
  },
};
