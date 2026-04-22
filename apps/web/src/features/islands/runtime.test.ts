import { describe, expect, it, vi } from 'vitest';
import {
  bootstrapHostIslands,
  hasHostIslandBoot,
  loadHostIslandModule,
  readHostIslandContract,
  type HostIslandMountContext,
  type HostIslandModule,
} from './runtime';

describe('host island runtime', () => {
  it('reads island instructions and inline data from host-rendered html', () => {
    document.body.innerHTML = `
      <div
        data-island-id="admin-screen:blog-dashboard"
        data-island-component="frontend.route:root"
      ></div>
      <script type="application/json" id="admin-preload:blog-dashboard">{"pageId":"blog-dashboard","path":"/admin/x/blog/dashboard","plugin":"blog","sdkVersion":"1.0.0"}</script>
      <script
        type="application/json"
        data-island-boot="admin-screen:blog-dashboard"
        data-module="/plugins/blog/admin/main.js"
      >{"entryId":"post-1"}</script>
    `;

    const contract = readHostIslandContract(document);

    expect(hasHostIslandBoot(document)).toBe(true);
    expect(contract.inlineData['admin-preload:blog-dashboard']).toEqual({
      pageId: 'blog-dashboard',
      path: '/admin/x/blog/dashboard',
      plugin: 'blog',
      sdkVersion: '1.0.0',
    });
    expect(contract.instructions).toHaveLength(1);
    expect(contract.instructions[0]).toMatchObject({
      islandId: 'admin-screen:blog-dashboard',
      component: 'frontend.route:root',
      moduleUrl: '/plugins/blog/admin/main.js',
      props: { entryId: 'post-1' },
    });
  });

  it('rejects boot entries without a matching mount node', () => {
    document.body.innerHTML = `
      <script
        type="application/json"
        data-island-boot="admin-screen:missing"
        data-module="/plugins/blog/admin/main.js"
      >{"entryId":"post-1"}</script>
    `;

    expect(() => readHostIslandContract(document)).toThrow('缺少对应的 mount 节点');
  });

  it('passes inline data and boot props to mounted island modules', async () => {
    document.body.innerHTML = `
      <div
        data-island-id="admin-screen:blog-dashboard"
        data-island-component="frontend.route:root"
      ></div>
      <script type="application/json" id="admin-preload:blog-dashboard">{"pageId":"blog-dashboard","path":"/admin/x/blog/dashboard","plugin":"blog","sdkVersion":"1.0.0"}</script>
      <script
        type="application/json"
        data-island-boot="admin-screen:blog-dashboard"
        data-module="/plugins/blog/admin/main.js"
      >{"entryId":"post-1"}</script>
    `;

    const mount = vi.fn<(context: HostIslandMountContext) => void>();
    const loadModule = vi.fn<(moduleUrl: string) => Promise<HostIslandModule>>(async () => ({
      mount,
    }));

    await bootstrapHostIslands(document, loadModule);

    expect(loadModule).toHaveBeenCalledWith('/plugins/blog/admin/main.js');
    expect(mount).toHaveBeenCalledWith(
      expect.objectContaining({
        pluginName: 'blog',
        contributionId: 'blog-dashboard',
        contributionKind: 'route',
        fullPath: '/admin/x/blog/dashboard',
        sdkVersion: '1.0.0',
        islandId: 'admin-screen:blog-dashboard',
        component: 'frontend.route:root',
        props: { entryId: 'post-1' },
        pageMode: null,
        inlineData: {
          'admin-preload:blog-dashboard': {
            pageId: 'blog-dashboard',
            path: '/admin/x/blog/dashboard',
            plugin: 'blog',
            sdkVersion: '1.0.0',
          },
        },
        apiClient: expect.any(Object),
        queryClient: expect.any(Object),
        auth: expect.objectContaining({
          user: null,
          isAdmin: false,
          isMember: false,
          refresh: expect.any(Function),
          logout: expect.any(Function),
        }),
        navigation: expect.objectContaining({
          pathname: '/admin/plugins',
          search: '',
          hash: '',
          navigate: expect.any(Function),
        }),
        logger: expect.objectContaining({
          info: expect.any(Function),
          warn: expect.any(Function),
          error: expect.any(Function),
        }),
      }),
    );
  });

  it('warns when plugin apiVersion differs from shell sdkVersion', async () => {
    document.body.innerHTML = `
      <div
        data-island-id="admin-screen:blog-dashboard"
        data-island-component="frontend.route:root"
      ></div>
      <script type="application/json" id="admin-preload:blog-dashboard">{"pageId":"blog-dashboard","path":"/admin/x/blog/dashboard","plugin":"blog","sdkVersion":"1.0.0","mode":"compatibility"}</script>
      <script
        type="application/json"
        data-island-boot="admin-screen:blog-dashboard"
        data-module="/plugins/blog/admin/main.js"
      >{"entryId":"post-1"}</script>
    `;

    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    const loadModule = vi.fn<(moduleUrl: string) => Promise<HostIslandModule>>(async () => ({
      apiVersion: '2.0.0',
      mount: vi.fn(),
    }));

    await bootstrapHostIslands(document, loadModule);

    expect(warn).toHaveBeenCalledWith(
      '[host-island:blog:blog-dashboard]',
      '插件模块声明 apiVersion=2.0.0，当前宿主为 1.0.0',
    );

    warn.mockRestore();
  });

  it('rejects cross-origin island modules before importing', async () => {
    await expect(loadHostIslandModule('https://cdn.example.com/blog.js')).rejects.toThrow(
      '不是同源 URL',
    );
  });
});
