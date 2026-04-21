import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { settingsApi } from '@/lib/api';
import { qk } from '@/lib/query-keys';

export function useSettingSchemas() {
  return useQuery({
    queryKey: qk.settings.schemas,
    queryFn: () => settingsApi.listSchemas(),
  });
}

export function useSettings(namespace: string) {
  return useQuery({
    queryKey: qk.settings.ns(namespace),
    queryFn: () => settingsApi.get(namespace),
  });
}

export function useSetSetting() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({
      namespace,
      key,
      value,
    }: {
      namespace: string;
      key: string;
      value: unknown;
    }) => settingsApi.set(namespace, key, value),
    onSuccess: (_res, variables) =>
      qc.invalidateQueries({ queryKey: qk.settings.ns(variables.namespace) }),
  });
}

export function useDeleteSetting() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ namespace, key }: { namespace: string; key: string }) =>
      settingsApi.delete(namespace, key),
    onSuccess: (_res, variables) =>
      qc.invalidateQueries({ queryKey: qk.settings.ns(variables.namespace) }),
  });
}
