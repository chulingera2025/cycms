import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { invalidateAdminExtensionQueries } from '@/features/admin-extensions/invalidation';
import { reportAdminExtensionEvent } from '@/features/admin-extensions/telemetry';
import { pluginsApi } from '@/lib/api';
import { qk } from '@/lib/query-keys';

export function usePlugins() {
  return useQuery({ queryKey: qk.plugins.list, queryFn: () => pluginsApi.list() });
}

type PluginAction = 'install' | 'enable' | 'disable' | 'uninstall';

export function usePluginAction(action: PluginAction) {
  const queryClient = useQueryClient();
  return useMutation<void, Error, string>({
    mutationFn: async (name: string) => {
      switch (action) {
        case 'install':
          await pluginsApi.install(name);
          return;
        case 'enable':
          await pluginsApi.enable(name);
          return;
        case 'disable':
          await pluginsApi.disable(name);
          return;
        case 'uninstall':
          await pluginsApi.uninstall(name);
          return;
      }
    },
    onSuccess: async (_, name) => {
      await invalidateAdminExtensionQueries(queryClient);
      reportAdminExtensionEvent({
        source: 'host',
        level: 'info',
        eventName: `plugin.${action}.success`,
        message: `插件 ${name} ${action} 操作成功`,
        pluginName: name,
        detail: { action },
      });
    },
    onError: (error, name) => {
      reportAdminExtensionEvent({
        source: 'host',
        level: 'error',
        eventName: `plugin.${action}.error`,
        message: `插件 ${name} ${action} 操作失败：${error.message}`,
        pluginName: name,
        detail: { action },
      });
    },
  });
}
