import { useMemo } from 'react';
import { useLocation, useNavigate, useParams } from 'react-router-dom';
import { Alert, Button, Card, Descriptions, Result, Skeleton, Space } from 'antd';
import { useAdminExtensions } from '@/features/admin-extensions';
import {
  ModuleHostBoundary,
  PluginModuleHost,
} from '@/features/admin-extensions/module-host';

function normalizeTailPath(tail?: string) {
  const normalized = (tail ?? '').replace(/^\/+|\/+$/g, '');
  return normalized ? `/${normalized}` : '/';
}

export default function PluginNamespacePage() {
  const navigate = useNavigate();
  const location = useLocation();
  const params = useParams();
  const {
    bootstrap,
    degraded,
    dismissRevisionChange,
    error,
    getPlugin,
    findRoute,
    findSettingsPage,
    isLoading,
    revision,
    revisionChange,
  } = useAdminExtensions();

  const pluginName = params.plugin ?? '';
  const tailPath = normalizeTailPath(params['*']);

  const plugin = getPlugin(pluginName);
  const route = findRoute(pluginName, tailPath);
  const settingsPage = findSettingsPage(pluginName, tailPath);
  const contribution = route
    ? {
        id: route.id,
        kind: 'route' as const,
        fullPath: route.fullPath,
        moduleUrl: route.moduleUrl,
        styles: route.styles,
      }
    : settingsPage
      ? {
          id: `${pluginName}:settings:${settingsPage.page.path}`,
          kind: 'settingsPage' as const,
          fullPath: settingsPage.page.fullPath,
          moduleUrl: settingsPage.page.moduleUrl,
          styles: settingsPage.page.styles,
        }
      : null;

  const title = route?.title ?? (settingsPage ? `${pluginName} 设置` : '插件页面');
  const staleAfterRevisionChange = Boolean(revisionChange) && (!plugin || !contribution);

  const descriptionItems = useMemo(
    () => [
      { key: 'plugin', label: '插件', children: pluginName || '-' },
      { key: 'version', label: '版本', children: plugin?.version ?? '-' },
      {
        key: 'path',
        label: '命中路径',
        children: settingsPage?.page.fullPath ?? route?.fullPath ?? location.pathname,
      },
      {
        key: 'type',
        label: '贡献类型',
        children: settingsPage ? 'custom_settings_page' : route?.kind ?? '-',
      },
      {
        key: 'module',
        label: '模块地址',
        children: settingsPage?.page.moduleUrl ?? route?.moduleUrl ?? '-',
      },
      {
        key: 'styles',
        label: '样式资源',
        children: (settingsPage?.page.styles ?? route?.styles ?? []).join(', ') || '无',
      },
    ],
    [location.pathname, plugin?.version, pluginName, route, settingsPage],
  );

  if (isLoading && !plugin) {
    return (
      <div className="p-6">
        <Skeleton active paragraph={{ rows: 6 }} />
      </div>
    );
  }

  if (!pluginName) {
    return (
      <Result
        status="404"
        title="缺少插件命名空间"
        subTitle="当前路由没有包含插件标识。"
        extra={<Button onClick={() => navigate('/admin/plugins')}>返回插件管理</Button>}
      />
    );
  }

  if (!plugin || !contribution) {
    return (
      <div className="p-6">
        <Result
          status="warning"
          title={staleAfterRevisionChange ? '插件页面已失效' : '插件页面已不可用'}
          subTitle={
            staleAfterRevisionChange
              ? `扩展注册表已从 ${revisionChange?.previousRevision ?? '-'} 更新为 ${revisionChange?.currentRevision ?? revision ?? '-'}，当前页面对应的插件贡献已经被宿主回收。`
              : '该插件可能已经被禁用、卸载，或当前用户已失去访问该贡献的权限。'
          }
          extra={
            <Space>
              <Button type="primary" onClick={() => navigate('/admin/plugins')}>
                返回插件管理
              </Button>
              <Button onClick={() => navigate('/admin/dashboard')}>返回仪表盘</Button>
            </Space>
          }
        />
      </div>
    );
  }

  return (
    <div className="space-y-4 p-6">
      <div>
        <h1 className="m-0 text-xl font-semibold text-text">{title}</h1>
        <p className="mt-1 text-sm text-text-muted">
          当前已进入插件命名空间路由，宿主已根据 bootstrap registry 解析到对应贡献元数据。
        </p>
      </div>

      {degraded && (
        <Alert
          type="warning"
          showIcon
          message="插件扩展注册表处于降级模式"
          description={error?.message ?? '最近一次 bootstrap 刷新失败，当前页面展示的是最后一次成功加载的路由元数据。'}
        />
      )}

      {revisionChange && (
        <Alert
          type="warning"
          showIcon
          closable
          onClose={dismissRevisionChange}
          message="插件扩展注册表已更新"
          description={`当前管理会话检测到注册表已从 ${revisionChange.previousRevision} 更新为 ${revisionChange.currentRevision}。若插件已被禁用或卸载，宿主会主动回收旧页面并阻止继续挂载失效模块。`}
        />
      )}

      <Alert
        type="success"
        showIcon
        message="三期模块宿主已开始接管插件页面"
        description="当前命名空间页会按 bootstrap 提供的 moduleUrl 和 styles 真正加载插件前端模块，并在路由失效或页面卸载时执行清理。"
      />

      <Card>
        <Descriptions column={1} items={descriptionItems} />
      </Card>

      <Card title="插件模块">
        <ModuleHostBoundary resetKey={`${pluginName}:${contribution.id}:${contribution.moduleUrl}`}>
          <PluginModuleHost
            pluginName={pluginName}
            contributionId={contribution.id}
            contributionKind={contribution.kind}
            fullPath={contribution.fullPath}
            sdkVersion={bootstrap?.shellSdkVersion ?? '1.0.0'}
            moduleUrl={contribution.moduleUrl}
            styles={contribution.styles}
          />
        </ModuleHostBoundary>
      </Card>

      <Space>
        <Button type="primary" onClick={() => navigate('/admin/plugins')}>
          返回插件管理
        </Button>
        <Button onClick={() => navigate('/admin/settings')}>返回系统设置</Button>
      </Space>
    </div>
  );
}