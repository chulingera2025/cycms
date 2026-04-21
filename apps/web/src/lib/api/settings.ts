import { api } from './client';
import type { PluginSchema, SettingsEntry } from '@/types';

const BASE = '/api/v1/settings';

export const settingsApi = {
  listSchemas(): Promise<PluginSchema[]> {
    return api.get<PluginSchema[]>(`${BASE}/schemas`);
  },

  get(namespace: string): Promise<SettingsEntry[]> {
    return api.get<SettingsEntry[]>(`${BASE}/${namespace}`);
  },

  set(namespace: string, key: string, value: unknown): Promise<SettingsEntry> {
    return api.put<SettingsEntry>(`${BASE}/${namespace}/${key}`, { value });
  },

  delete(namespace: string, key: string): Promise<void> {
    return api.delete(`${BASE}/${namespace}/${key}`);
  },
};
