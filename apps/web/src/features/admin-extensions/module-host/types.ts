import type { api } from '@/lib/api';
import type { QueryClient } from '@tanstack/react-query';
import type { NavigateFunction } from 'react-router-dom';
import type { AuthContextValue } from '@/stores/auth';
import type { FieldDefinition } from '@/types';

export type AdminPluginApiClient = typeof api;

export interface AdminPluginLogger {
  info: (...args: unknown[]) => void;
  warn: (...args: unknown[]) => void;
  error: (...args: unknown[]) => void;
}

export interface AdminPluginNavigation {
  pathname: string;
  search: string;
  hash: string;
  navigate: NavigateFunction;
}

export interface AdminPluginMountContext {
  container: HTMLElement;
  pluginName: string;
  contributionId: string;
  contributionKind: 'route' | 'settingsPage' | 'fieldRenderer' | 'slot';
  fullPath: string;
  sdkVersion: string;
  apiClient: AdminPluginApiClient;
  queryClient: QueryClient;
  auth: Pick<
    AuthContextValue,
    'user' | 'isAdmin' | 'isMember' | 'refresh' | 'logout'
  >;
  navigation: AdminPluginNavigation;
  logger: AdminPluginLogger;
  slot?: {
    slotId: string;
    contentTypeApiId: string;
    entryId?: string;
    mode: 'create' | 'edit';
    values: Record<string, unknown>;
    dirtyFields: string[];
    isDirty: boolean;
    validationErrors: Record<string, string | null>;
    setFieldValue: (apiId: string, value: unknown) => void;
    getFieldValue: (apiId: string) => unknown;
    setFieldError: (apiId: string, message: string | null) => void;
    getFieldError: (apiId: string) => string | null;
    validateField: (apiId: string) => string | null;
  };
  fieldRenderer?: {
    field: FieldDefinition;
    value: unknown;
    setValue: (value: unknown) => void;
    contentTypeApiId: string;
    entryId?: string;
    mode: 'create' | 'edit';
    dirty: boolean;
    validationError: string | null;
    setValidationError: (message: string | null) => void;
    validate: () => string | null;
  };
}

export interface AdminPluginMountHandle {
  unmount?: () => void | Promise<void>;
  update?: (context: AdminPluginMountContext) => void | Promise<void>;
}

export interface AdminPluginModule {
  apiVersion?: string;
  mount: (
    context: AdminPluginMountContext,
  ) => void | AdminPluginMountHandle | Promise<void | AdminPluginMountHandle>;
  unmount?: (context: AdminPluginMountContext) => void | Promise<void>;
}