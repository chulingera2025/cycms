import {
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react';
import { useQuery } from '@tanstack/react-query';
import { adminExtensionsApi } from '@/lib/api';
import { qk } from '@/lib/query-keys';
import { useAuth } from '@/stores/auth';
import {
  RegistryContext,
  type AdminExtensionRegistryValue,
} from './context';
import type { AdminExtensionBootstrap } from '@/types';

const ZONE_ORDER: Record<string, number> = {
  content: 10,
  plugins: 20,
  system: 30,
  settings: 40,
};

function normalizeExtensionPath(path: string) {
  const normalized = `/${path.replace(/^\/+|\/+$/g, '')}`.replace(/\/+/g, '/');
  return normalized === '/' ? '/' : normalized;
}

function routeMatches(targetPath: string, routePath: string) {
  const target = normalizeExtensionPath(targetPath);
  const route = normalizeExtensionPath(routePath);
  if (target === route) {
    return true;
  }
  return route !== '/' && target.startsWith(`${route}/`);
}

export function AdminExtensionRegistryProvider({ children }: { children: ReactNode }) {
  const { isAdmin, loading } = useAuth();
  const [lastGoodBootstrap, setLastGoodBootstrap] = useState<AdminExtensionBootstrap | null>(null);
  const enabled = isAdmin && !loading;

  const bootstrapQuery = useQuery({
    queryKey: qk.adminExtensions.bootstrap,
    queryFn: () => adminExtensionsApi.bootstrap(),
    enabled,
    staleTime: 0,
    refetchOnReconnect: true,
    refetchInterval: enabled ? 15_000 : false,
  });

  useEffect(() => {
    if (bootstrapQuery.data) {
      setLastGoodBootstrap(bootstrapQuery.data);
    }
  }, [bootstrapQuery.data]);

  useEffect(() => {
    if (!enabled) {
      setLastGoodBootstrap(null);
    }
  }, [enabled]);

  const effectiveBootstrap = bootstrapQuery.data ?? lastGoodBootstrap;

  const value = useMemo<AdminExtensionRegistryValue>(() => {
    const plugins = effectiveBootstrap?.plugins ?? [];
    const diagnostics = effectiveBootstrap?.diagnostics ?? [];
    const pluginMap = new Map(plugins.map((plugin) => [plugin.name, plugin]));

    const menuItems = plugins
      .flatMap((plugin) =>
        plugin.menus.map((menu) => ({
          pluginName: plugin.name,
          pluginVersion: plugin.version,
          id: menu.id,
          label: menu.label,
          zone: menu.zone,
          icon: menu.icon,
          order: menu.order,
          to: menu.to,
          fullPath: menu.fullPath,
        })),
      )
      .sort((left, right) => {
        const zoneCompare = (ZONE_ORDER[left.zone] ?? 100) - (ZONE_ORDER[right.zone] ?? 100);
        if (zoneCompare !== 0) {
          return zoneCompare;
        }
        if (left.order !== right.order) {
          return left.order - right.order;
        }
        const labelCompare = left.label.localeCompare(right.label, 'zh-CN');
        if (labelCompare !== 0) {
          return labelCompare;
        }
        return left.pluginName.localeCompare(right.pluginName, 'zh-CN');
      });

    const settingsNamespaces = plugins
      .flatMap((plugin) =>
        plugin.settings
          ? [
              {
                namespace: plugin.settings.namespace,
                pluginName: plugin.name,
                pluginVersion: plugin.version,
                contribution: plugin.settings,
              },
            ]
          : [],
      )
      .sort((left, right) => left.namespace.localeCompare(right.namespace, 'zh-CN'));

    return {
      bootstrap: effectiveBootstrap ?? null,
      revision: effectiveBootstrap?.revision ?? null,
      plugins,
      diagnostics,
      menuItems,
      settingsNamespaces,
      isLoading: enabled && bootstrapQuery.isLoading && !effectiveBootstrap,
      degraded: bootstrapQuery.isError,
      error: bootstrapQuery.error instanceof Error ? bootstrapQuery.error : null,
      getPlugin(pluginName) {
        return pluginMap.get(pluginName) ?? null;
      },
      findRoute(pluginName, path) {
        const plugin = pluginMap.get(pluginName);
        if (!plugin) {
          return null;
        }
        return (
          [...plugin.routes]
            .sort((left, right) => right.path.length - left.path.length)
            .find((route) => routeMatches(path, route.path)) ?? null
        );
      },
      findSettingsPage(pluginName, path) {
        const plugin = pluginMap.get(pluginName);
        const settings = plugin?.settings;
        const page = settings?.customPage;
        if (!plugin || !settings || !page || !routeMatches(path, page.path)) {
          return null;
        }
        return { plugin, settings, page };
      },
      getSettingsNamespace(namespace) {
        return (
          settingsNamespaces.find((entry) => entry.namespace === namespace) ?? null
        );
      },
    };
  }, [bootstrapQuery.error, bootstrapQuery.isError, bootstrapQuery.isLoading, effectiveBootstrap, enabled]);

  return <RegistryContext value={value}>{children}</RegistryContext>;
}