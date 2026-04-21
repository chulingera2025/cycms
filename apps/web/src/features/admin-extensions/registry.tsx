import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { adminExtensionsApi } from '@/lib/api';
import { qk } from '@/lib/query-keys';
import { notify } from '@/lib/toast';
import { reportAdminExtensionEvent } from './telemetry';
import { useAuth } from '@/stores/auth';
import {
  RegistryContext,
  type AdminExtensionRegistryValue,
} from './context';
import {
  broadcastAdminExtensionRevision,
  invalidateAdminExtensionQueries,
  readStoredAdminExtensionRevision,
  subscribeToAdminExtensionRevision,
} from './invalidation';
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

function matchContentType(contentTypeApiIds: string[], contentTypeApiId?: string) {
  if (!contentTypeApiIds.length) {
    return true;
  }
  if (!contentTypeApiId) {
    return false;
  }
  return contentTypeApiIds.includes(contentTypeApiId);
}

export function AdminExtensionRegistryProvider({ children }: { children: ReactNode }) {
  const { isAdmin, loading } = useAuth();
  const queryClient = useQueryClient();
  const [lastGoodBootstrap, setLastGoodBootstrap] = useState<AdminExtensionBootstrap | null>(null);
  const [revisionChange, setRevisionChange] = useState<AdminExtensionRegistryValue['revisionChange']>(null);
  const knownRevisionRef = useRef<string | null>(readStoredAdminExtensionRevision());
  const enabled = isAdmin && !loading;

  const dismissRevisionChange = useCallback(() => {
    setRevisionChange(null);
  }, []);

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
      setRevisionChange(null);
    }
  }, [enabled]);

  useEffect(() => {
    const nextRevision = bootstrapQuery.data?.revision;
    if (!nextRevision) {
      return;
    }

    const previousRevision = knownRevisionRef.current;
    const changedAt = new Date().toISOString();
    if (previousRevision && previousRevision !== nextRevision) {
      setRevisionChange({ previousRevision, currentRevision: nextRevision, changedAt });
      notify.warning(
        '插件扩展注册表已更新，后台菜单、命名空间路由和设置页将按最新状态重算。',
      );
      reportAdminExtensionEvent({
        source: 'host',
        level: 'info',
        eventName: 'registry.revision.changed',
        message: `插件扩展注册表已从 ${previousRevision} 更新为 ${nextRevision}`,
        detail: { previousRevision, currentRevision: nextRevision, changedAt },
      });
    }

    if (previousRevision !== nextRevision) {
      knownRevisionRef.current = nextRevision;
      broadcastAdminExtensionRevision(nextRevision, changedAt);
    }
  }, [bootstrapQuery.data?.revision]);

  useEffect(() => {
    if (!enabled) {
      return () => undefined;
    }

    return subscribeToAdminExtensionRevision((event) => {
      const previousRevision = knownRevisionRef.current;
      if (previousRevision === event.revision) {
        return;
      }

      if (previousRevision) {
        setRevisionChange({
          previousRevision,
          currentRevision: event.revision,
          changedAt: event.changedAt,
        });
        notify.warning(
          '检测到其他管理会话更新了插件注册表，当前页面将同步最新扩展状态。',
        );
        reportAdminExtensionEvent({
          source: 'host',
          level: 'info',
          eventName: 'registry.revision.synced',
          message: `检测到其他会话将插件扩展注册表更新为 ${event.revision}`,
          detail: {
            previousRevision,
            currentRevision: event.revision,
            changedAt: event.changedAt,
          },
        });
      }

      knownRevisionRef.current = event.revision;
      void invalidateAdminExtensionQueries(queryClient);
    });
  }, [enabled, queryClient]);

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

    const fieldRenderers = plugins
      .flatMap((plugin) =>
        plugin.fieldRenderers.map((contribution) => ({
          pluginName: plugin.name,
          pluginVersion: plugin.version,
          contribution,
        })),
      )
      .sort((left, right) => {
        const typeCompare = left.contribution.typeName.localeCompare(
          right.contribution.typeName,
          'zh-CN',
        );
        if (typeCompare !== 0) {
          return typeCompare;
        }
        return left.pluginName.localeCompare(right.pluginName, 'zh-CN');
      });

    const fieldRendererMap = new Map(
      fieldRenderers.map((entry) => [entry.contribution.typeName, entry]),
    );

    const slots = plugins
      .flatMap((plugin) =>
        plugin.slots.map((contribution) => ({
          pluginName: plugin.name,
          pluginVersion: plugin.version,
          contribution,
        })),
      )
      .sort((left, right) => {
        if (left.contribution.slot !== right.contribution.slot) {
          return left.contribution.slot.localeCompare(right.contribution.slot, 'zh-CN');
        }
        if (left.contribution.order !== right.contribution.order) {
          return left.contribution.order - right.contribution.order;
        }
        return left.pluginName.localeCompare(right.pluginName, 'zh-CN');
      });

    return {
      bootstrap: effectiveBootstrap ?? null,
      revision: effectiveBootstrap?.revision ?? null,
      revisionChange,
      plugins,
      diagnostics,
      menuItems,
      settingsNamespaces,
      fieldRenderers,
      slots,
      isLoading: enabled && bootstrapQuery.isLoading && !effectiveBootstrap,
      degraded: bootstrapQuery.isError,
      error: bootstrapQuery.error instanceof Error ? bootstrapQuery.error : null,
      dismissRevisionChange,
      getPlugin(pluginName) {
        return pluginMap.get(pluginName) ?? null;
      },
      getFieldRenderer(typeName) {
        return fieldRendererMap.get(typeName) ?? null;
      },
      getSlots(slotId, contentTypeApiId) {
        return slots.filter(
          (entry) =>
            entry.contribution.slot === slotId &&
            matchContentType(entry.contribution.match.contentTypeApiIds, contentTypeApiId),
        );
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
  }, [
    bootstrapQuery.error,
    bootstrapQuery.isError,
    bootstrapQuery.isLoading,
    dismissRevisionChange,
    effectiveBootstrap,
    enabled,
    revisionChange,
  ]);

  return <RegistryContext value={value}>{children}</RegistryContext>;
}