import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { Skeleton } from 'antd';
import { useLocation, useNavigate } from 'react-router-dom';
import { reportAdminExtensionEvent } from '@/features/admin-extensions/telemetry';
import { api } from '@/lib/api';
import { useAuth } from '@/stores/auth';
import type { FieldDefinition } from '@/types';
import { loadAdminPluginModule, retainPluginStyles } from './loader';
import type {
  AdminPluginLogger,
  AdminPluginModule,
  AdminPluginMountContext,
  AdminPluginMountHandle,
} from './types';

interface PluginFieldRendererHostProps {
  pluginName: string;
  contributionId: string;
  sdkVersion: string;
  moduleUrl: string;
  styles: string[];
  field: FieldDefinition;
  value: unknown;
  onChange: (value: unknown) => void;
  contentTypeApiId: string;
  entryId?: string;
  mode: 'create' | 'edit';
  dirty: boolean;
  validationError: string | null;
  setValidationError: (message: string | null) => void;
  validate: () => string | null;
  onFatalError?: (error: Error) => void;
}

function asError(error: unknown) {
  if (error instanceof Error) {
    return error;
  }
  return new Error(typeof error === 'string' ? error : '字段扩展模块加载失败');
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

export function PluginFieldRendererHost({
  pluginName,
  contributionId,
  sdkVersion,
  moduleUrl,
  styles,
  field,
  value,
  onChange,
  contentTypeApiId,
  entryId,
  mode,
  dirty,
  validationError,
  setValidationError,
  validate,
  onFatalError,
}: PluginFieldRendererHostProps) {
  const auth = useAuth();
  const queryClient = useQueryClient();
  const location = useLocation();
  const navigate = useNavigate();
  const containerRef = useRef<HTMLDivElement | null>(null);
  const mountedModuleRef = useRef<AdminPluginModule | null>(null);
  const mountHandleRef = useRef<AdminPluginMountHandle | null>(null);
  const currentContextRef = useRef<AdminPluginMountContext | null>(null);
  const onChangeRef = useRef(onChange);
  const fieldRef = useRef(field);
  const valueRef = useRef(value);
  const contentTypeApiIdRef = useRef(contentTypeApiId);
  const entryIdRef = useRef(entryId);
  const modeRef = useRef(mode);
  const dirtyRef = useRef(dirty);
  const validationErrorRef = useRef(validationError);
  const setValidationErrorRef = useRef(setValidationError);
  const validateRef = useRef(validate);
  const [isLoading, setIsLoading] = useState(true);
  const [failed, setFailed] = useState(false);

  const logger = useMemo(() => createLogger(pluginName, contributionId), [contributionId, pluginName]);
  const styleKey = useMemo(() => [...new Set(styles)].sort().join('|'), [styles]);

  useEffect(() => {
    onChangeRef.current = onChange;
  }, [onChange]);

  useEffect(() => {
    fieldRef.current = field;
    valueRef.current = value;
    contentTypeApiIdRef.current = contentTypeApiId;
    entryIdRef.current = entryId;
    modeRef.current = mode;
    dirtyRef.current = dirty;
    validationErrorRef.current = validationError;
    setValidationErrorRef.current = setValidationError;
    validateRef.current = validate;
  }, [contentTypeApiId, dirty, entryId, field, mode, setValidationError, validate, validationError, value]);

  const buildContext = useCallback(
    (container: HTMLElement): AdminPluginMountContext => ({
      container,
      pluginName,
      contributionId,
      contributionKind: 'fieldRenderer',
      fullPath: location.pathname,
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
      fieldRenderer: {
        field: fieldRef.current,
        value: valueRef.current,
        setValue(nextValue) {
          onChangeRef.current(nextValue);
        },
        contentTypeApiId: contentTypeApiIdRef.current,
        entryId: entryIdRef.current,
        mode: modeRef.current,
        dirty: dirtyRef.current,
        validationError: validationErrorRef.current,
        setValidationError(message) {
          setValidationErrorRef.current(message);
        },
        validate() {
          return validateRef.current();
        },
      },
    }),
    [
      auth.isAdmin,
      auth.isMember,
      auth.logout,
      auth.refresh,
      auth.user,
      contributionId,
      location.hash,
      location.pathname,
      location.search,
      logger,
      navigate,
      pluginName,
      queryClient,
      sdkVersion,
    ],
  );

  useEffect(() => {
    const nextContainer = containerRef.current;
    if (!nextContainer) {
      return;
    }
    const container: HTMLElement = nextContainer;

    let disposed = false;
    let cleanedUp = false;
    let releaseStyles: (() => void) | null = null;

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
          logger.warn('字段渲染器 mount handle 执行 unmount 失败', cleanupError);
        }
      }
      mountHandleRef.current = null;

      const mountedModule = mountedModuleRef.current;
      if (mountedModule?.unmount && currentContextRef.current) {
        try {
          await mountedModule.unmount(currentContextRef.current);
        } catch (cleanupError) {
          logger.warn('字段渲染器模块执行 unmount 失败', cleanupError);
        }
      }
      mountedModuleRef.current = null;
      currentContextRef.current = null;

      container.innerHTML = '';
      releaseStyles?.();
      releaseStyles = null;
    };

    async function mountRenderer() {
      container.innerHTML = '';
      setIsLoading(true);
      setFailed(false);
      reportAdminExtensionEvent({
        source: 'host',
        level: 'info',
        eventName: 'module.load.start',
        message: `开始加载字段渲染器 ${pluginName}:${contributionId}`,
        pluginName,
        contributionId,
        contributionKind: 'fieldRenderer',
        fullPath: location.pathname,
        detail: { moduleUrl, contentTypeApiId: contentTypeApiIdRef.current },
      });

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

        const mountContext = buildContext(container);
        currentContextRef.current = mountContext;
        mountedModuleRef.current = pluginModule;
        const mountResult = await pluginModule.mount(mountContext);
        mountHandleRef.current = isMountHandle(mountResult) ? mountResult : null;

        if (disposed) {
          await runCleanup();
          return;
        }

        setIsLoading(false);
        reportAdminExtensionEvent({
          source: 'host',
          level: 'info',
          eventName: 'module.mount.success',
          message: `字段渲染器 ${pluginName}:${contributionId} 已挂载`,
          pluginName,
          contributionId,
          contributionKind: 'fieldRenderer',
          fullPath: location.pathname,
        });
      } catch (mountError) {
        const nextError = asError(mountError);
        logger.error('字段渲染器模块加载或挂载失败', nextError);
        setFailed(true);
        setIsLoading(false);
        reportAdminExtensionEvent({
          source: 'host',
          level: 'error',
          eventName: 'module.mount.error',
          message: `字段渲染器 ${pluginName}:${contributionId} 挂载失败：${nextError.message}`,
          pluginName,
          contributionId,
          contributionKind: 'fieldRenderer',
          fullPath: location.pathname,
        });
        onFatalError?.(nextError);
        await runCleanup();
      }
    }

    void mountRenderer();

    return () => {
      disposed = true;
      reportAdminExtensionEvent({
        source: 'host',
        level: 'info',
        eventName: 'module.unmount.start',
        message: `字段渲染器 ${pluginName}:${contributionId} 开始卸载`,
        pluginName,
        contributionId,
        contributionKind: 'fieldRenderer',
        fullPath: location.pathname,
      });
      void runCleanup();
    };
  }, [buildContext, contributionId, location.pathname, logger, moduleUrl, onFatalError, pluginName, styleKey, styles]);

  useEffect(() => {
    const container = containerRef.current;
    const mountHandle = mountHandleRef.current;
    if (!container || !mountHandle?.update || failed) {
      return;
    }

    const nextContext = buildContext(container);
    currentContextRef.current = nextContext;
    void mountHandle.update(nextContext);
  }, [
    buildContext,
    contentTypeApiId,
    entryId,
    failed,
    field,
    mode,
    dirty,
    validationError,
    value,
  ]);

  if (failed) {
    return null;
  }

  return (
    <div className="space-y-2">
      {isLoading && <Skeleton.Input active block style={{ height: 40 }} />}
      <div ref={containerRef} className="min-h-10" />
    </div>
  );
}