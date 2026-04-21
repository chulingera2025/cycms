import { useEffect, useMemo, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { Alert, Skeleton } from 'antd';
import { useLocation, useNavigate } from 'react-router-dom';
import { api } from '@/lib/api';
import { useAuth } from '@/stores/auth';
import { loadAdminPluginModule, retainPluginStyles } from './loader';
import type {
  AdminPluginLogger,
  AdminPluginModule,
  AdminPluginMountContext,
  AdminPluginMountHandle,
} from './types';

interface PluginModuleHostProps {
  pluginName: string;
  contributionId: string;
  contributionKind: 'route' | 'settingsPage';
  fullPath: string;
  sdkVersion: string;
  moduleUrl: string;
  styles: string[];
}

function asError(error: unknown) {
  if (error instanceof Error) {
    return error;
  }
  return new Error(typeof error === 'string' ? error : '插件模块加载失败');
}

function isMountHandle(value: unknown): value is AdminPluginMountHandle {
  return Boolean(value) && typeof value === 'object';
}

function createLogger(pluginName: string, contributionId: string): AdminPluginLogger {
  const prefix = `[admin-plugin:${pluginName}:${contributionId}]`;
  return {
    info: (...args) => console.info(prefix, ...args),
    warn: (...args) => console.warn(prefix, ...args),
    error: (...args) => console.error(prefix, ...args),
  };
}

export function PluginModuleHost({
  pluginName,
  contributionId,
  contributionKind,
  fullPath,
  sdkVersion,
  moduleUrl,
  styles,
}: PluginModuleHostProps) {
  const auth = useAuth();
  const queryClient = useQueryClient();
  const location = useLocation();
  const navigate = useNavigate();
  const containerRef = useRef<HTMLDivElement | null>(null);
  const mountedModuleRef = useRef<AdminPluginModule | null>(null);
  const mountHandleRef = useRef<AdminPluginMountHandle | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [versionWarning, setVersionWarning] = useState<string | null>(null);

  const styleKey = useMemo(() => [...new Set(styles)].sort().join('|'), [styles]);
  const logger = useMemo(() => createLogger(pluginName, contributionId), [contributionId, pluginName]);

  useEffect(() => {
    const nextContainer = containerRef.current;
    if (!nextContainer) {
      return;
    }
    const container: HTMLElement = nextContainer;

    let disposed = false;
    let cleanedUp = false;
    let releaseStyles: (() => void) | null = null;
    let currentContext: AdminPluginMountContext | null = null;

    const runCleanup = async () => {
      if (cleanedUp) {
        return;
      }
      cleanedUp = true;

      const mountHandle = mountHandleRef.current;
      if (mountHandle?.unmount) {
        try {
          await mountHandle.unmount();
        } catch (cleanupError) {
          logger.warn('插件 mount handle 执行 unmount 失败', cleanupError);
        }
      }
      mountHandleRef.current = null;

      const mountedModule = mountedModuleRef.current;
      if (mountedModule?.unmount && currentContext) {
        try {
          await mountedModule.unmount(currentContext);
        } catch (cleanupError) {
          logger.warn('插件模块执行 unmount 失败', cleanupError);
        }
      }
      mountedModuleRef.current = null;

      container.innerHTML = '';
      releaseStyles?.();
      releaseStyles = null;
    };

    async function mountModule() {
      container.innerHTML = '';
      setIsLoading(true);
      setError(null);
      setVersionWarning(null);

      try {
        releaseStyles = await retainPluginStyles(styles);
        if (disposed) {
          await runCleanup();
          return;
        }

        const pluginModule = await loadAdminPluginModule(moduleUrl);
        if (disposed) {
          await runCleanup();
          return;
        }

        if (pluginModule.apiVersion && pluginModule.apiVersion !== sdkVersion) {
          const warning = `插件模块声明 apiVersion=${pluginModule.apiVersion}，当前宿主为 ${sdkVersion}`;
          setVersionWarning(warning);
          logger.warn(warning);
        }

        const mountContext: AdminPluginMountContext = {
          container,
          pluginName,
          contributionId,
          contributionKind,
          fullPath,
          sdkVersion,
          apiClient: api,
          queryClient,
          auth: {
            user: auth.user,
            isAdmin: auth.isAdmin,
            isMember: auth.isMember,
            refresh: auth.refresh,
            logout: auth.logout,
          },
          navigation: {
            pathname: location.pathname,
            search: location.search,
            hash: location.hash,
            navigate,
          },
          logger,
        };
        currentContext = mountContext;

        mountedModuleRef.current = pluginModule;
        const mountResult = await pluginModule.mount(mountContext);
        mountHandleRef.current = isMountHandle(mountResult) ? mountResult : null;

        if (disposed) {
          await runCleanup();
          return;
        }

        setIsLoading(false);
      } catch (mountError) {
        const nextError = asError(mountError);
        logger.error('插件模块加载或挂载失败', nextError);
        setError(nextError);
        setIsLoading(false);
        await runCleanup();
      }
    }

    void mountModule();

    return () => {
      disposed = true;
      void runCleanup();
    };
  }, [
    auth.isAdmin,
    auth.isMember,
    auth.logout,
    auth.refresh,
    auth.user,
    contributionId,
    contributionKind,
    fullPath,
    location.hash,
    location.pathname,
    location.search,
    logger,
    moduleUrl,
    navigate,
    pluginName,
    queryClient,
    sdkVersion,
    styleKey,
    styles,
  ]);

  return (
    <div className="space-y-4">
      {versionWarning && (
        <Alert
          type="warning"
          showIcon
          message="插件模块 SDK 声明与宿主存在偏差"
          description={versionWarning}
        />
      )}

      {error && (
        <Alert
          type="error"
          showIcon
          message="插件模块加载失败"
          description={error.message}
        />
      )}

      {isLoading && <Skeleton active paragraph={{ rows: 4 }} />}

      <div
        ref={containerRef}
        className="min-h-[160px] rounded border border-border bg-surface p-4"
      />
    </div>
  );
}