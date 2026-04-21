export const qk = {
  auth: {
    me: ['auth', 'me'] as const,
  },
  contentTypes: {
    list: ['content-types'] as const,
    detail: (apiId: string) => ['content-types', apiId] as const,
  },
  content: {
    all: (typeApiId: string) => ['content', typeApiId] as const,
    list: (typeApiId: string, params?: Record<string, unknown>) =>
      ['content', typeApiId, 'list', params ?? {}] as const,
    detail: (typeApiId: string, id: string) =>
      ['content', typeApiId, 'detail', id] as const,
    revisions: (typeApiId: string, id: string) =>
      ['content', typeApiId, 'detail', id, 'revisions'] as const,
  },
  media: {
    list: (params?: Record<string, unknown>) => ['media', params ?? {}] as const,
    detail: (id: string) => ['media', id] as const,
  },
  users: {
    list: ['users'] as const,
    detail: (id: string) => ['users', id] as const,
  },
  roles: {
    list: ['roles'] as const,
    detail: (id: string) => ['roles', id] as const,
    permissions: ['roles', 'permissions'] as const,
  },
  plugins: {
    list: ['plugins'] as const,
    detail: (name: string) => ['plugins', name] as const,
  },
  adminExtensions: {
    bootstrap: ['admin-extensions', 'bootstrap'] as const,
    diagnostics: ['admin-extensions', 'diagnostics'] as const,
  },
  settings: {
    schemas: ['settings', 'schemas'] as const,
    ns: (namespace: string) => ['settings', namespace] as const,
  },
  publicContent: {
    types: ['public', 'content-types'] as const,
    list: (typeApiId: string, params?: Record<string, unknown>) =>
      ['public', 'content', typeApiId, params ?? {}] as const,
    detail: (typeApiId: string, idOrSlug: string) =>
      ['public', 'content', typeApiId, idOrSlug] as const,
    search: (q: string, typeApiId?: string) =>
      ['public', 'search', q, typeApiId ?? ''] as const,
  },
};
