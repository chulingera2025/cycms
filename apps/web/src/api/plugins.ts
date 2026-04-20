import { api } from './client';
import type { Plugin } from '@/types';

const BASE = '/api/v1/plugins';

export const pluginsApi = {
  list(): Promise<Plugin[]> {
    return api.get<Plugin[]>(BASE);
  },

  install(name: string): Promise<Plugin> {
    return api.post<Plugin>(`${BASE}/${name}/install`);
  },

  enable(name: string): Promise<Plugin> {
    return api.post<Plugin>(`${BASE}/${name}/enable`);
  },

  disable(name: string): Promise<Plugin> {
    return api.post<Plugin>(`${BASE}/${name}/disable`);
  },

  uninstall(name: string): Promise<void> {
    return api.delete(`${BASE}/${name}`);
  },
};
