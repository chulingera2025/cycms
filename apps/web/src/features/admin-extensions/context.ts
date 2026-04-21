import { createContext, useContext } from 'react';
import type {
  AdminExtensionBootstrap,
  BootstrapPlugin,
  BootstrapRouteContribution,
  BootstrapSettingsContribution,
  BootstrapSettingsPage,
  ExtensionDiagnostic,
} from '@/types';

export interface AdminExtensionMenuEntry {
  pluginName: string;
  pluginVersion: string;
  id: string;
  label: string;
  zone: string;
  icon?: string;
  order: number;
  to: string;
  fullPath: string;
}

export interface AdminExtensionSettingsNamespace {
  namespace: string;
  pluginName: string;
  pluginVersion: string;
  contribution: BootstrapSettingsContribution;
}

interface MatchedSettingsPage {
  plugin: BootstrapPlugin;
  settings: BootstrapSettingsContribution;
  page: BootstrapSettingsPage;
}

export interface AdminExtensionRegistryValue {
  bootstrap: AdminExtensionBootstrap | null;
  revision: string | null;
  plugins: BootstrapPlugin[];
  diagnostics: ExtensionDiagnostic[];
  menuItems: AdminExtensionMenuEntry[];
  settingsNamespaces: AdminExtensionSettingsNamespace[];
  isLoading: boolean;
  degraded: boolean;
  error: Error | null;
  getPlugin: (pluginName: string) => BootstrapPlugin | null;
  findRoute: (pluginName: string, path: string) => BootstrapRouteContribution | null;
  findSettingsPage: (pluginName: string, path: string) => MatchedSettingsPage | null;
  getSettingsNamespace: (namespace: string) => AdminExtensionSettingsNamespace | null;
}

export const RegistryContext = createContext<AdminExtensionRegistryValue | null>(null);

export function useAdminExtensions() {
  const value = useContext(RegistryContext);
  if (!value) {
    throw new Error('useAdminExtensions must be used within AdminExtensionRegistryProvider');
  }
  return value;
}