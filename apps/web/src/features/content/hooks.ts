import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { contentApi } from '@/lib/api';
import { qk } from '@/lib/query-keys';
import type { CreateEntryInput, UpdateEntryInput } from '@/types';

export function useContentList(
  typeApiId: string | undefined,
  params: Record<string, string>,
) {
  return useQuery({
    queryKey: typeApiId ? qk.content.list(typeApiId, params) : ['content', 'noop'],
    queryFn: () => contentApi.list(typeApiId!, params),
    enabled: Boolean(typeApiId),
  });
}

export function useRevisions(typeApiId: string | undefined, id: string | undefined) {
  return useQuery({
    queryKey:
      typeApiId && id ? qk.content.revisions(typeApiId, id) : ['content', 'noop-rev'],
    queryFn: () => contentApi.listRevisions(typeApiId!, id!),
    enabled: Boolean(typeApiId && id),
  });
}

export function useCreateEntry(typeApiId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateEntryInput) => contentApi.create(typeApiId, input),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.content.all(typeApiId) }),
  });
}

export function useUpdateEntry(typeApiId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateEntryInput }) =>
      contentApi.update(typeApiId, id, input),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.content.all(typeApiId) }),
  });
}

export function useDeleteEntry(typeApiId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => contentApi.delete(typeApiId, id),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.content.all(typeApiId) }),
  });
}

export function usePublishEntry(typeApiId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => contentApi.publish(typeApiId, id),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.content.all(typeApiId) }),
  });
}

export function useUnpublishEntry(typeApiId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => contentApi.unpublish(typeApiId, id),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.content.all(typeApiId) }),
  });
}

export function useRollbackRevision(typeApiId: string, id: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (version: number) => contentApi.rollbackRevision(typeApiId, id, version),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.content.all(typeApiId) });
      qc.invalidateQueries({ queryKey: qk.content.revisions(typeApiId, id) });
    },
  });
}
