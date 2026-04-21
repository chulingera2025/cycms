import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { mediaApi } from '@/lib/api';
import { qk } from '@/lib/query-keys';

export function useMediaList(params: Record<string, string>) {
  return useQuery({
    queryKey: qk.media.list(params),
    queryFn: () => mediaApi.list(params),
  });
}

export function useMedia(id: string | null | undefined) {
  return useQuery({
    queryKey: id ? qk.media.detail(id) : ['media', 'noop'],
    queryFn: () => mediaApi.get(id!),
    enabled: Boolean(id),
  });
}

export function useUploadMedia() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (file: File) => mediaApi.upload(file),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['media'] }),
  });
}

export function useDeleteMedia() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => mediaApi.delete(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['media'] }),
  });
}
