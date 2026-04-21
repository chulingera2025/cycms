import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { Alert, Skeleton } from 'antd';
import { useLocation, useNavigate } from 'react-router-dom';
import { reportAdminExtensionEvent } from '@/features/admin-extensions/telemetry';
import { api } from '@/lib/api';
import { useAuth } from '@/stores/auth';
import { loadAdminPluginModule, retainPluginStyles } from './loader';
import type {
  AdminPluginLogger,
  AdminPluginModule,
  AdminPluginMountContext,
  AdminPluginMountHandle,
} from './types';

interface PluginSlotHostProps {
  pluginName: string;
  contributionId: string;
  slotId: string;
  sdkVersion: string;
  moduleUrl: string;
  styles: string[];
  contentTypeApiId: string;
  values: Record<string, unknown>;
  dirtyFields: string[];
  validationErrors: Record<string, string | null>;
  setFieldValue: (apiId: string, value: unknown) => void;
  setFieldError: (apiId: string, message: string | null) => void;
  getFieldError: (apiId: string) => string | null;
  validateField: (apiId: string) => string | null;
  entryId?: string;
  mode: 'create' | 'edit';
}

function asError(error: unknown) {
  if (error instanceof Error) {
    return error;
  }
  return new Error(typeof error === 'string' ? error : '插件 slot 加载失败');
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

export function PluginSlotHost({
  pluginName,
  contributionId,
  slotId,
  sdkVersion,
  moduleUrl,
  styles,
  contentTypeApiId,
  values,
  dirtyFields,
  validationErrors,
  setFieldValue,
  setFieldError,
  getFieldError,
  validateField,
  entryId,
  mode,
}: PluginSlotHostProps) {
  const auth = useAuth();
  const queryClient = useQueryClient();
  const location = useLocation();
  const navigate = useNavigate();
  const containerRef = useRef<HTMLDivElement | null>(null);
  const mountedModuleRef = useRef<AdminPluginModule | null>(null);
  const mountHandleRef = useRef<AdminPluginMountHandle | null>(null);
  const currentContextRef = useRef<AdminPluginMountContext | null>(null);
  const valuesRef = useRef(values);
  const setFieldValueRef = useRef(setFieldValue);
  const contentTypeApiIdRef = useRef(contentTypeApiId);
  const entryIdRef = useRef(entryId);
  const modeRef = useRef(mode);
  const dirtyFieldsRef = useRef(dirtyFields);
  const validationErrorsRef = useRef(validationErrors);
  const setFieldErrorRef = useRef(setFieldError);
  const getFieldErrorRef = useRef(getFieldError);
  const validateFieldRef = useRef(validateField);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  const logger = useMemo(() => createLogger(pluginName, contributionId), [contributionId, pluginName]);
  const styleKey = useMemo(() => [...new Set(styles)].sort().join('|'), [styles]);

  useEffect(() => {
    valuesRef.current = values;
    setFieldValueRef.current = setFieldValue;
    contentTypeApiIdRef.current = contentTypeApiId;
    entryIdRef.current = entryId;
    modeRef.current = mode;
    dirtyFieldsRef.current = dirtyFields;
    validationErrorsRef.current = validationErrors;
    setFieldErrorRef.current = setFieldError;
    getFieldErrorRef.current = getFieldError;
    validateFieldRef.current = validateField;
  }, [contentTypeApiId, dirtyFields, entryId, getFieldError, mode, setFieldError, setFieldValue, validateField, validationErrors, values]);

  const buildContext = useCallback(
    (container: HTMLElement): AdminPluginMountContext => ({
      container,
      pluginName,
      contributionId,
      contributionKind: 'slot',
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
      slot: {
        slotId,
        contentTypeApiId: contentTypeApiIdRef.current,
        entryId: entryIdRef.current,
        mode: modeRef.current,
        values: valuesRef.current,
        dirtyFields: dirtyFieldsRef.current,
        isDirty: dirtyFieldsRef.current.length > 0,
        validationErrors: validationErrorsRef.current,
        setFieldValue(apiId, value) {
          setFieldValueRef.current(apiId, value);
        },
        getFieldValue(apiId) {
          return valuesRef.current[apiId];
        },
        setFieldError(apiId, message) {
          setFieldErrorRef.current(apiId, message);
        },
        getFieldError(apiId) {
          return getFieldErrorRef.current(apiId);
        },
        validateField(apiId) {
          return validateFieldRef.current(apiId);
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
      slotId,
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
          logger.warn('slot mount handle 执行 unmount 失败', cleanupError);
        }
      }
      mountHandleRef.current = null;

      const mountedModule = mountedModuleRef.current;
      if (mountedModule?.unmount && currentContextRef.current) {
        try {
          await mountedModule.unmount(currentContextRef.current);
        } catch (cleanupError) {
          logger.warn('slot 模块执行 unmount 失败', cleanupError);
        }
      }
      mountedModuleRef.current = null;
      currentContextRef.current = null;

      container.innerHTML = '';
      releaseStyles?.();
      releaseStyles = null;
    };

    async function mountSlot() {
      container.innerHTML = '';
      setIsLoading(true);
      setError(null);
      reportAdminExtensionEvent({
        source: 'host',
        level: 'info',
        eventName: 'module.load.start',
        message: `开始加载 slot ${pluginName}:${contributionId}`,
        pluginName,
        contributionId,
        contributionKind: 'slot',
        fullPath: location.pathname,
        detail: { moduleUrl, slotId },
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
          message: `slot ${pluginName}:${contributionId} 已挂载`,
          pluginName,
          contributionId,
          contributionKind: 'slot',
          fullPath: location.pathname,
        });
      } catch (mountError) {
        const nextError = asError(mountError);
        logger.error('slot 模块加载或挂载失败', nextError);
        setError(nextError);
        setIsLoading(false);
        reportAdminExtensionEvent({
          source: 'host',
          level: 'error',
          eventName: 'module.mount.error',
          message: `slot ${pluginName}:${contributionId} 挂载失败：${nextError.message}`,
          pluginName,
          contributionId,
          contributionKind: 'slot',
          fullPath: location.pathname,
        });
        await runCleanup();
      }
    }

    void mountSlot();

    return () => {
      disposed = true;
      reportAdminExtensionEvent({
        source: 'host',
        level: 'info',
        eventName: 'module.unmount.start',
        message: `slot ${pluginName}:${contributionId} 开始卸载`,
        pluginName,
        contributionId,
        contributionKind: 'slot',
        fullPath: location.pathname,
      });
      void runCleanup();
    };
  }, [buildContext, contributionId, location.pathname, logger, moduleUrl, pluginName, slotId, styleKey, styles]);

  useEffect(() => {
    const container = containerRef.current;
    const mountHandle = mountHandleRef.current;
    if (!container || !mountHandle?.update) {
      return;
    }

    const nextContext = buildContext(container);
    currentContextRef.current = nextContext;
    void mountHandle.update(nextContext);
  }, [buildContext, contentTypeApiId, dirtyFields, entryId, mode, validationErrors, values]);

  return (
    <div className="space-y-2 rounded border border-border bg-surface p-3">
      {error && (
        <Alert
          type="error"
          showIcon
          message="插件侧边栏挂载失败"
          description={error.message}
        />
      )}
      {isLoading && <Skeleton active paragraph={{ rows: 3 }} />}
      <div ref={containerRef} className="min-h-10" />
    </div>
  );
}