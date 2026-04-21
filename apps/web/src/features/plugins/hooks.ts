import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { invalidateAdminExtensionQueries } from '@/features/admin-extensions/invalidation';
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
    onSuccess: async () => {
      await invalidateAdminExtensionQueries(queryClient);
    },
  });
}
