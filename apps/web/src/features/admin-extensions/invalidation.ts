import type { QueryClient } from '@tanstack/react-query';
import { qk } from '@/lib/query-keys';

const REVISION_KEY = 'cycms.adminExtensions.revision';
const REVISION_EVENT_KEY = 'cycms.adminExtensions.revisionEvent';

export interface AdminExtensionRevisionEvent {
  revision: string;
  changedAt: string;
}

function hasStorageSupport() {
  return typeof window !== 'undefined' && typeof window.localStorage !== 'undefined';
}

function parseRevisionEvent(raw: string | null): AdminExtensionRevisionEvent | null {
  if (!raw) {
    return null;
  }

  try {
    const parsed = JSON.parse(raw) as Partial<AdminExtensionRevisionEvent>;
    if (typeof parsed.revision !== 'string' || typeof parsed.changedAt !== 'string') {
      return null;
    }
    return {
      revision: parsed.revision,
      changedAt: parsed.changedAt,
    };
  } catch {
    return null;
  }
}

export function readStoredAdminExtensionRevision() {
  if (!hasStorageSupport()) {
    return null;
  }
  return window.localStorage.getItem(REVISION_KEY);
}

export function broadcastAdminExtensionRevision(
  revision: string,
  changedAt = new Date().toISOString(),
) {
  if (!hasStorageSupport()) {
    return;
  }

  const payload = JSON.stringify({ revision, changedAt });
  window.localStorage.setItem(REVISION_KEY, revision);
  window.localStorage.setItem(REVISION_EVENT_KEY, payload);
}

export function subscribeToAdminExtensionRevision(
  listener: (event: AdminExtensionRevisionEvent) => void,
) {
  if (!hasStorageSupport()) {
    return () => undefined;
  }

  const handleStorage = (storageEvent: StorageEvent) => {
    if (storageEvent.key !== REVISION_EVENT_KEY) {
      return;
    }

    const event = parseRevisionEvent(storageEvent.newValue);
    if (event) {
      listener(event);
    }
  };

  window.addEventListener('storage', handleStorage);
  return () => window.removeEventListener('storage', handleStorage);
}

export async function invalidateAdminExtensionQueries(queryClient: QueryClient) {
  await Promise.all([
    queryClient.invalidateQueries({ queryKey: qk.adminExtensions.bootstrap }),
    queryClient.invalidateQueries({ queryKey: qk.adminExtensions.diagnostics }),
    queryClient.invalidateQueries({ queryKey: ['settings'] }),
    queryClient.invalidateQueries({ queryKey: qk.plugins.list }),
  ]);
}