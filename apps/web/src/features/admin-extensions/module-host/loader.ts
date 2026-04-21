import type { AdminPluginModule } from './types';

interface StyleRecord {
  element: HTMLLinkElement;
  refs: number;
  ready: Promise<void>;
}

const moduleCache = new Map<string, Promise<AdminPluginModule>>();
const styleRegistry = new Map<string, StyleRecord>();

function normalizeImportedModule(moduleUrl: string, imported: unknown): AdminPluginModule {
  const namespace =
    imported && typeof imported === 'object'
      ? (imported as { default?: unknown }).default ?? imported
      : imported;

  if (typeof namespace === 'function') {
    return { mount: namespace as AdminPluginModule['mount'] };
  }

  if (namespace && typeof namespace === 'object') {
    const candidate = namespace as Partial<AdminPluginModule>;
    if (typeof candidate.mount === 'function') {
      return {
        apiVersion: typeof candidate.apiVersion === 'string' ? candidate.apiVersion : undefined,
        mount: candidate.mount as AdminPluginModule['mount'],
        unmount: typeof candidate.unmount === 'function' ? candidate.unmount : undefined,
      };
    }
  }

  throw new Error(
    `插件模块 ${moduleUrl} 未导出可用的 mount(context) 合约`,
  );
}

function ensureStyle(url: string): StyleRecord {
  const existing = styleRegistry.get(url);
  if (existing) {
    existing.refs += 1;
    return existing;
  }

  const element = document.createElement('link');
  element.rel = 'stylesheet';
  element.href = url;
  element.dataset.cycmsPluginStyle = url;

  const ready = new Promise<void>((resolve, reject) => {
    element.addEventListener('load', () => resolve(), { once: true });
    element.addEventListener(
      'error',
      () => reject(new Error(`加载插件样式失败: ${url}`)),
      { once: true },
    );
  });

  document.head.appendChild(element);

  const record = { element, refs: 1, ready };
  styleRegistry.set(url, record);
  return record;
}

function releaseStyle(url: string) {
  const record = styleRegistry.get(url);
  if (!record) {
    return;
  }

  record.refs -= 1;
  if (record.refs <= 0) {
    record.element.remove();
    styleRegistry.delete(url);
  }
}

export async function retainPluginStyles(styleUrls: string[]) {
  const urls = [...new Set(styleUrls)];
  if (!urls.length) {
    return () => undefined;
  }

  const records = urls.map((url) => ensureStyle(url));
  try {
    await Promise.all(records.map((record) => record.ready));
  } catch (error) {
    for (const url of urls) {
      releaseStyle(url);
    }
    throw error;
  }

  let released = false;
  return () => {
    if (released) {
      return;
    }
    released = true;
    for (const url of urls) {
      releaseStyle(url);
    }
  };
}

export async function loadAdminPluginModule(moduleUrl: string) {
  const cached = moduleCache.get(moduleUrl);
  if (cached) {
    return cached;
  }

  const loading = import(/* @vite-ignore */ moduleUrl)
    .then((imported) => normalizeImportedModule(moduleUrl, imported))
    .catch((error) => {
      moduleCache.delete(moduleUrl);
      throw error;
    });

  moduleCache.set(moduleUrl, loading);
  return loading;
}