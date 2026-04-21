import { useQuery } from '@tanstack/react-query';
import { publicApi } from '@/lib/api';
import { qk } from '@/lib/query-keys';

export function usePublicContentTypes() {
  return useQuery({
    queryKey: qk.publicContent.types,
    queryFn: () => publicApi.listContentTypes(),
  });
}

export function usePublicContentList(
  typeApiId: string | undefined,
  params: Record<string, string>,
) {
  return useQuery({
    queryKey: typeApiId ? qk.publicContent.list(typeApiId, params) : ['public', 'noop'],
    queryFn: () => publicApi.listContent(typeApiId!, params),
    enabled: Boolean(typeApiId),
  });
}

export function usePublicContentDetail(
  typeApiId: string | undefined,
  idOrSlug: string | undefined,
) {
  return useQuery({
    queryKey:
      typeApiId && idOrSlug
        ? qk.publicContent.detail(typeApiId, idOrSlug)
        : ['public', 'noop-detail'],
    queryFn: () => publicApi.getContent(typeApiId!, idOrSlug!),
    enabled: Boolean(typeApiId && idOrSlug),
  });
}
