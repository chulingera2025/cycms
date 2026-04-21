import { adminExtensionsApi } from '@/lib/api/admin-extensions';
import type { AdminExtensionClientEventPayload } from '@/types';

export function reportAdminExtensionEvent(payload: AdminExtensionClientEventPayload) {
  void adminExtensionsApi.recordEvent(payload).catch(() => undefined);
}