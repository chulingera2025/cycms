import { api } from './client';
import type {
  User,
  CreateUserInput,
  UpdateUserInput,
  Role,
  CreateRoleInput,
  UpdateRoleInput,
  Permission,
} from '@/types';

const USERS_BASE = '/api/v1/users';
const ROLES_BASE = '/api/v1/roles';

export const usersApi = {
  list(): Promise<User[]> {
    return api.get<User[]>(USERS_BASE);
  },

  get(id: string): Promise<User> {
    return api.get<User>(`${USERS_BASE}/${id}`);
  },

  create(data: CreateUserInput): Promise<User> {
    return api.post<User>(USERS_BASE, data);
  },

  update(id: string, data: UpdateUserInput): Promise<User> {
    return api.put<User>(`${USERS_BASE}/${id}`, data);
  },

  delete(id: string): Promise<void> {
    return api.delete(`${USERS_BASE}/${id}`);
  },
};

export const rolesApi = {
  list(): Promise<Role[]> {
    return api.get<Role[]>(ROLES_BASE);
  },

  get(id: string): Promise<Role> {
    return api.get<Role>(`${ROLES_BASE}/${id}`);
  },

  create(data: CreateRoleInput): Promise<Role> {
    return api.post<Role>(ROLES_BASE, data);
  },

  update(id: string, data: UpdateRoleInput): Promise<Role> {
    return api.put<Role>(`${ROLES_BASE}/${id}`, data);
  },

  delete(id: string): Promise<void> {
    return api.delete(`${ROLES_BASE}/${id}`);
  },

  listPermissions(): Promise<Permission[]> {
    return api.get<Permission[]>(`${ROLES_BASE}/permissions`);
  },
};
