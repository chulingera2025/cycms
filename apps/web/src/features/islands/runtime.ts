import { createPath, type NavigateOptions, type To } from 'react-router-dom';
import { loadAdminPluginModule } from '@/features/admin-extensions/module-host/loader';
import type {
  AdminPluginLogger,
  AdminPluginMountHandle,
  AdminPluginMountContext,
  AdminPluginNavigation,
} from '@/features/admin-extensions/module-host/types';
import { api, clearTokens, getAccessToken } from '@/lib/api';
import { authApi } from '@/lib/api/auth';
import { queryClient } from '@/lib/query-client';
import type { User } from '@/types';

interface HostIslandBootEntry {
  islandId: string;
  moduleUrl: string;
  props: unknown;
}

interface AdminPagePreload {
  pageId?: string;
  path?: string;
  plugin?: string;
  mode?: string;
  sdkVersion?: string;
}

const ADMIN_ROLES = ['super_admin', 'editor', 'author'];
const DEFAULT_ADMIN_SHELL_SDK_VERSION = '1.0.0';

export interface HostIslandRuntimeContract {
  inlineData: Record<string, unknown>;
  instructions: HostIslandInstruction[];
}

export interface HostIslandInstruction {
  islandId: string;
  component: string;
  container: HTMLElement;
  moduleUrl: string;
  props: unknown;
}

export interface HostIslandMountContext extends AdminPluginMountContext {
  islandId: string;
  component: string;
  props: unknown;
  inlineData: Record<string, unknown>;
  pageMode: string | null;
}

export interface HostIslandModule {
  apiVersion?: string;
  mount:
    | ((context: HostIslandMountContext) => void | AdminPluginMountHandle)
    | ((context: HostIslandMountContext) => Promise<void | AdminPluginMountHandle>);
  unmount?: (context: HostIslandMountContext) => void | Promise<void>;
}

function parseJsonScript(script: HTMLScriptElement, label: string) {
  const payload = script.textContent?.trim() ?? '';
  if (!payload) {
    return null;
  }

  try {
    return JSON.parse(payload) as unknown;
  } catch (error) {
    throw new Error(`${label} 包含无效 JSON`, { cause: error });
  }
}

function collectInlineData(doc: Document) {
  const inlineData: Record<string, unknown> = {};

  for (const script of doc.querySelectorAll<HTMLScriptElement>(
    'script[type="application/json"][id]:not([data-island-boot])',
  )) {
    inlineData[script.id] = parseJsonScript(script, `inline data ${script.id}`);
  }

  return inlineData;
}

function collectIslandMounts(doc: Document) {
  const mounts = new Map<string, { component: string; container: HTMLElement }>();

  for (const node of doc.querySelectorAll<HTMLElement>('[data-island-id][data-island-component]')) {
    const islandId = node.dataset.islandId;
    const component = node.dataset.islandComponent;
    if (!islandId || !component) {
      continue;
    }

    if (mounts.has(islandId)) {
      throw new Error(`发现重复的 island mount: ${islandId}`);
    }

    mounts.set(islandId, { component, container: node });
  }

  return mounts;
}

function collectIslandBootEntries(doc: Document) {
  const boots: HostIslandBootEntry[] = [];
  const seenIslandIds = new Set<string>();

  for (const script of doc.querySelectorAll<HTMLScriptElement>(
    'script[type="application/json"][data-island-boot][data-module]',
  )) {
    const islandId = script.dataset.islandBoot;
    const moduleUrl = script.dataset.module;
    if (!islandId || !moduleUrl) {
      continue;
    }

    if (seenIslandIds.has(islandId)) {
      throw new Error(`发现重复的 island boot: ${islandId}`);
    }

    seenIslandIds.add(islandId);
    boots.push({
      islandId,
      moduleUrl,
      props: parseJsonScript(script, `island boot ${islandId}`),
    });
  }

  return boots;
}

function deriveAuthFlags(user: User | null) {
  const roles = user?.roles ?? [];
  return {
    isAdmin: roles.some((role) => ADMIN_ROLES.includes(role)),
    isMember: user !== null && !roles.some((role) => ADMIN_ROLES.includes(role)),
  };
}

function createLogger(pluginName: string, contributionId: string): AdminPluginLogger {
  const prefix = `[host-island:${pluginName}:${contributionId}]`;
  return {
    info: (...args) => console.info(prefix, ...args),
    warn: (...args) => console.warn(prefix, ...args),
    error: (...args) => console.error(prefix, ...args),
  };
}

function createNavigation(): AdminPluginNavigation {
  const navigate = ((to: To | number, options?: NavigateOptions) => {
    if (typeof to === 'number') {
      window.history.go(to);
      return;
    }

    const href = typeof to === 'string' ? to : createPath(to);
    if (options?.replace) {
      window.location.replace(href);
      return;
    }

    window.location.assign(href);
  }) as AdminPluginNavigation['navigate'];

  return {
    pathname: window.location.pathname,
    search: window.location.search,
    hash: window.location.hash,
    navigate,
  };
}

async function createAuthBridge(): Promise<AdminPluginMountContext['auth']> {
  const auth: AdminPluginMountContext['auth'] = {
    user: null,
    isAdmin: false,
    isMember: false,
    refresh: async () => {
      if (!getAccessToken()) {
        auth.user = null;
        auth.isAdmin = false;
        auth.isMember = false;
        return;
      }

      try {
        const user = await authApi.me();
        const flags = deriveAuthFlags(user);
        auth.user = user;
        auth.isAdmin = flags.isAdmin;
        auth.isMember = flags.isMember;
      } catch {
        clearTokens();
        auth.user = null;
        auth.isAdmin = false;
        auth.isMember = false;
      }
    },
    logout: () => {
      clearTokens();
      auth.user = null;
      auth.isAdmin = false;
      auth.isMember = false;
    },
  };

  await auth.refresh();
  return auth;
}

function readAdminPreload(contract: HostIslandRuntimeContract, instruction: HostIslandInstruction) {
  const pageId = instruction.islandId.startsWith('admin-screen:')
    ? instruction.islandId.slice('admin-screen:'.length)
    : instruction.islandId;
  const payload = contract.inlineData[`admin-preload:${pageId}`];
  if (!payload || typeof payload !== 'object') {
    throw new Error(`island ${instruction.islandId} 缺少对应的 admin preload 数据`);
  }

  return payload as AdminPagePreload;
}

function inferContributionKind(component: string): 'route' | 'settingsPage' {
  if (component.startsWith('frontend.settings:')) {
    return 'settingsPage';
  }

  return 'route';
}

function inferPluginName(path: string, preloadPlugin?: string) {
  if (preloadPlugin) {
    return preloadPlugin;
  }

  const segments = path.split('/').filter(Boolean);
  const pluginName = segments[2];
  if (!pluginName) {
    throw new Error(`无法从路径 ${path} 推断插件名`);
  }

  return pluginName;
}

function createMountContext(
  contract: HostIslandRuntimeContract,
  instruction: HostIslandInstruction,
  auth: AdminPluginMountContext['auth'],
): HostIslandMountContext {
  const preload = readAdminPreload(contract, instruction);
  const fullPath = preload.path ?? window.location.pathname;
  const contributionId = preload.pageId ?? instruction.islandId;
  const pluginName = inferPluginName(fullPath, preload.plugin);

  return {
    container: instruction.container,
    pluginName,
    contributionId,
    contributionKind: inferContributionKind(instruction.component),
    fullPath,
    sdkVersion: preload.sdkVersion ?? DEFAULT_ADMIN_SHELL_SDK_VERSION,
    apiClient: api,
    queryClient,
    auth,
    navigation: createNavigation(),
    logger: createLogger(pluginName, contributionId),
    islandId: instruction.islandId,
    component: instruction.component,
    props: instruction.props,
    inlineData: contract.inlineData,
    pageMode: preload.mode ?? null,
  };
}

export function hasHostIslandBoot(doc: Document = document) {
  return (
    doc.querySelector('script[type="application/json"][data-island-boot][data-module]') !== null
  );
}

export function readHostIslandContract(doc: Document = document): HostIslandRuntimeContract {
  const inlineData = collectInlineData(doc);
  const mounts = collectIslandMounts(doc);
  const instructions = collectIslandBootEntries(doc).map((boot) => {
    const mount = mounts.get(boot.islandId);
    if (!mount) {
      throw new Error(`island ${boot.islandId} 缺少对应的 mount 节点`);
    }

    return {
      islandId: boot.islandId,
      component: mount.component,
      container: mount.container,
      moduleUrl: boot.moduleUrl,
      props: boot.props,
    } satisfies HostIslandInstruction;
  });

  return { inlineData, instructions };
}

export async function loadHostIslandModule(moduleUrl: string) {
  return (await loadAdminPluginModule(moduleUrl)) as HostIslandModule;
}

export async function bootstrapHostIslands(
  doc: Document = document,
  loadModule: (moduleUrl: string) => Promise<HostIslandModule> = loadHostIslandModule,
) {
  const contract = readHostIslandContract(doc);
  const auth = await createAuthBridge();

  return Promise.all(
    contract.instructions.map(async (instruction) => {
      const module = await loadModule(instruction.moduleUrl);
      const context = createMountContext(contract, instruction, auth);

      if (module.apiVersion && module.apiVersion !== context.sdkVersion) {
        context.logger.warn(
          `插件模块声明 apiVersion=${module.apiVersion}，当前宿主为 ${context.sdkVersion}`,
        );
      }

      return (await module.mount(context)) ?? {};
    }),
  );
}
