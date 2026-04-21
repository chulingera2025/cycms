import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { rolesApi } from '@/lib/api';
import { qk } from '@/lib/query-keys';
import type { CreateRoleInput, UpdateRoleInput } from '@/types';

export function useRoles() {
  return useQuery({ queryKey: qk.roles.list, queryFn: () => rolesApi.list() });
}

export function usePermissions() {
  return useQuery({
    queryKey: qk.roles.permissions,
    queryFn: () => rolesApi.listPermissions(),
  });
}

export function useCreateRole() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateRoleInput) => rolesApi.create(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.roles.list }),
  });
}

export function useUpdateRole() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateRoleInput }) =>
      rolesApi.update(id, input),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.roles.list }),
  });
}

export function useDeleteRole() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => rolesApi.delete(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.roles.list }),
  });
}
