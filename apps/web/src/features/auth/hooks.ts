import { useMutation, useQueryClient } from '@tanstack/react-query';
import { authApi, publicApi } from '@/lib/api';
import { qk } from '@/lib/query-keys';
import type { LoginInput, RegisterInput } from './schema';

export function useAdminLogin() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: LoginInput) => authApi.login(input),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: qk.auth.me }),
  });
}

export function useMemberLogin() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: LoginInput) => publicApi.login(input),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: qk.auth.me }),
  });
}

export function useRegister() {
  return useMutation({
    mutationFn: (input: Omit<RegisterInput, 'confirmPassword'>) => publicApi.register(input),
  });
}
