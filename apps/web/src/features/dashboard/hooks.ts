import { useQuery } from '@tanstack/react-query';
import { contentTypesApi, mediaApi, pluginsApi, usersApi } from '@/lib/api';

export function useStats() {
  const contentTypes = useQuery({
    queryKey: ['stats', 'content-types'],
    queryFn: () => contentTypesApi.list(),
  });
  const users = useQuery({ queryKey: ['stats', 'users'], queryFn: () => usersApi.list() });
  const media = useQuery({
    queryKey: ['stats', 'media'],
    queryFn: () => mediaApi.list({ page: '1', pageSize: '1' }),
  });
  const plugins = useQuery({
    queryKey: ['stats', 'plugins'],
    queryFn: () => pluginsApi.list(),
  });

  return {
    contentTypes: contentTypes.data?.length ?? 0,
    users: users.data?.length ?? 0,
    media: media.data?.total ?? 0,
    plugins: plugins.data?.filter((p) => p.status === 'enabled').length ?? 0,
    loading:
      contentTypes.isLoading ||
      users.isLoading ||
      media.isLoading ||
      plugins.isLoading,
  };
}
