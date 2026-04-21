import type { AdminPluginModule } from './types';

interface StyleRecord {
  element: HTMLLinkElement;
  refs: number;
  ready: Promise<void>;
}

const moduleCache = new Map<string, Promise<AdminPluginModule>>();
const styleRegistry = new Map<string, StyleRecord>();

function ensureSameOriginAssetUrl(assetUrl: string) {
  const normalized = new URL(assetUrl, window.location.origin);
  if (normalized.origin !== window.location.origin) {
    throw new Error(`插件资产 ${assetUrl} 不是同源 URL，宿主已拒绝加载`);
  }
  if (normalized.protocol !== 'http:' && normalized.protocol !== 'https:') {
    throw new Error(`插件资产 ${assetUrl} 使用了不受支持的协议 ${normalized.protocol}`);
  }
  return normalized.toString();
}

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
  const normalizedUrl = ensureSameOriginAssetUrl(url);
  const existing = styleRegistry.get(normalizedUrl);
  if (existing) {
    existing.refs += 1;
    return existing;
  }

  const element = document.createElement('link');
  element.rel = 'stylesheet';
  element.href = normalizedUrl;
  element.dataset.cycmsPluginStyle = normalizedUrl;

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
  styleRegistry.set(normalizedUrl, record);
  return record;
}

function releaseStyle(url: string) {
  const normalizedUrl = ensureSameOriginAssetUrl(url);
  const record = styleRegistry.get(normalizedUrl);
  if (!record) {
    return;
  }

  record.refs -= 1;
  if (record.refs <= 0) {
    record.element.remove();
    styleRegistry.delete(normalizedUrl);
  }
}

export async function retainPluginStyles(styleUrls: string[]) {
  const urls = [...new Set(styleUrls.map((url) => ensureSameOriginAssetUrl(url)))];
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
  const normalizedUrl = ensureSameOriginAssetUrl(moduleUrl);
  const cached = moduleCache.get(normalizedUrl);
  if (cached) {
    return cached;
  }

  const loading = import(/* @vite-ignore */ normalizedUrl)
    .then((imported) => normalizeImportedModule(normalizedUrl, imported))
    .catch((error) => {
      moduleCache.delete(normalizedUrl);
      throw error;
    });

  moduleCache.set(normalizedUrl, loading);
  return loading;
}