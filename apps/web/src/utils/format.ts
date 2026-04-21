export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export function formatDateTime(iso: string): string {
  return new Date(iso).toLocaleString('zh-CN');
}

export function resolveMediaUrl(storagePath: string): string {
  return storagePath.startsWith('/') ? storagePath : `/uploads/${storagePath}`;
}
