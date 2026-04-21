import { api } from './client';
import type {
  AdminExtensionBootstrap,
  AdminExtensionDiagnostics,
} from '@/types';

const BASE = '/api/v1/admin/extensions';

export const adminExtensionsApi = {
  bootstrap(): Promise<AdminExtensionBootstrap> {
    return api.get<AdminExtensionBootstrap>(`${BASE}/bootstrap`);
  },

  diagnostics(): Promise<AdminExtensionDiagnostics> {
    return api.get<AdminExtensionDiagnostics>(`${BASE}/diagnostics`);
  },
};