import { createContext, useContext } from 'react';
import type {
  AdminExtensionBootstrap,
  BootstrapFieldRendererContribution,
  BootstrapPlugin,
  BootstrapRouteContribution,
  BootstrapSlotContribution,
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

export interface AdminExtensionFieldRendererEntry {
  pluginName: string;
  pluginVersion: string;
  contribution: BootstrapFieldRendererContribution;
}

export interface AdminExtensionSlotEntry {
  pluginName: string;
  pluginVersion: string;
  contribution: BootstrapSlotContribution;
}

export interface AdminExtensionRevisionChange {
  previousRevision: string;
  currentRevision: string;
  changedAt: string;
}

interface MatchedSettingsPage {
  plugin: BootstrapPlugin;
  settings: BootstrapSettingsContribution;
  page: BootstrapSettingsPage;
}

export interface AdminExtensionRegistryValue {
  bootstrap: AdminExtensionBootstrap | null;
  revision: string | null;
  revisionChange: AdminExtensionRevisionChange | null;
  plugins: BootstrapPlugin[];
  diagnostics: ExtensionDiagnostic[];
  menuItems: AdminExtensionMenuEntry[];
  settingsNamespaces: AdminExtensionSettingsNamespace[];
  fieldRenderers: AdminExtensionFieldRendererEntry[];
  slots: AdminExtensionSlotEntry[];
  isLoading: boolean;
  degraded: boolean;
  error: Error | null;
  dismissRevisionChange: () => void;
  getPlugin: (pluginName: string) => BootstrapPlugin | null;
  getFieldRenderer: (typeName: string) => AdminExtensionFieldRendererEntry | null;
  getSlots: (slotId: string, contentTypeApiId?: string) => AdminExtensionSlotEntry[];
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