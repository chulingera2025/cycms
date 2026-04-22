import { useQuery } from '@tanstack/react-query';
import { publicApi } from '@/lib/api';
import { qk } from '@/lib/query-keys';
import {
  toBlogPostDetail,
  toBlogPostSummary,
  toBlogSiteSettings,
} from './blog';

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

export function useBlogPosts(limit = 12) {
  const params = {
    page: '1',
    pageSize: String(limit),
    sort: 'published_at:desc',
  };

  return useQuery({
    queryKey: qk.publicContent.list('post', params),
    queryFn: () => publicApi.listContent('post', params),
    select: (response) => response.data.map(toBlogPostSummary),
  });
}

export function useBlogPostIndex(page = 1, pageSize = 12, enabled = true) {
  const params = {
    page: String(page),
    pageSize: String(pageSize),
    sort: 'published_at:desc',
  };

  return useQuery({
    queryKey: qk.publicContent.list('post', params),
    queryFn: () => publicApi.listContent('post', params),
    enabled,
    select: (response) => ({
      data: response.data.map(toBlogPostSummary),
      meta: response.meta,
    }),
  });
}

export function useFeaturedBlogPosts(limit = 4) {
  const params = {
    page: '1',
    pageSize: String(limit),
    sort: 'published_at:desc',
    'filter[featured][eq]': 'true',
  };

  return useQuery({
    queryKey: qk.publicContent.list('post', params),
    queryFn: () => publicApi.listContent('post', params),
    select: (response) => response.data.map(toBlogPostSummary),
  });
}

export function useBlogPostDetail(idOrSlug: string | undefined, enabled = true) {
  return useQuery({
    queryKey: idOrSlug ? qk.publicContent.detail('post', idOrSlug) : ['public', 'blog', 'noop-detail'],
    queryFn: () => publicApi.getContent('post', idOrSlug!, ['categories', 'tags']),
    enabled: Boolean(idOrSlug) && enabled,
    select: toBlogPostDetail,
  });
}

export function useBlogSiteSettings() {
  const params = {
    page: '1',
    pageSize: '1',
    populate: 'featured_posts',
  };

  return useQuery({
    queryKey: qk.publicContent.list('site_settings', params),
    queryFn: () => publicApi.listContent('site_settings', params),
    select: (response) => toBlogSiteSettings(response.data[0] ?? null),
  });
}
