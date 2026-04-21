import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { contentTypesApi } from '@/lib/api';
import { qk } from '@/lib/query-keys';
import type { CreateContentTypeInput, UpdateContentTypeInput } from '@/types';

export function useContentTypes() {
  return useQuery({
    queryKey: qk.contentTypes.list,
    queryFn: () => contentTypesApi.list(),
  });
}

export function useContentType(apiId: string | undefined) {
  return useQuery({
    queryKey: apiId ? qk.contentTypes.detail(apiId) : qk.contentTypes.list,
    queryFn: () => contentTypesApi.get(apiId!),
    enabled: Boolean(apiId),
  });
}

export function useCreateContentType() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateContentTypeInput) => contentTypesApi.create(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.contentTypes.list }),
  });
}

export function useUpdateContentType() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ apiId, input }: { apiId: string; input: UpdateContentTypeInput }) =>
      contentTypesApi.update(apiId, input),
    onSuccess: (_res, variables) => {
      qc.invalidateQueries({ queryKey: qk.contentTypes.list });
      qc.invalidateQueries({ queryKey: qk.contentTypes.detail(variables.apiId) });
    },
  });
}

export function useDeleteContentType() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (apiId: string) => contentTypesApi.delete(apiId),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.contentTypes.list }),
  });
}
