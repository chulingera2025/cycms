import { useMemo } from 'react';
import { useLocation, useNavigate, useParams } from 'react-router-dom';
import { Alert, Button, Card, Descriptions, Result, Skeleton, Space } from 'antd';
import { useAdminExtensions } from '@/features/admin-extensions';

function normalizeTailPath(tail?: string) {
  const normalized = (tail ?? '').replace(/^\/+|\/+$/g, '');
  return normalized ? `/${normalized}` : '/';
}

export default function PluginNamespacePage() {
  const navigate = useNavigate();
  const location = useLocation();
  const params = useParams();
  const {
    degraded,
    error,
    getPlugin,
    findRoute,
    findSettingsPage,
    isLoading,
  } = useAdminExtensions();

  const pluginName = params.plugin ?? '';
  const tailPath = normalizeTailPath(params['*']);

  const plugin = getPlugin(pluginName);
  const route = findRoute(pluginName, tailPath);
  const settingsPage = findSettingsPage(pluginName, tailPath);

  const title = route?.title ?? (settingsPage ? `${pluginName} 设置` : '插件页面');

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

  if (!plugin || (!route && !settingsPage)) {
    return (
      <div className="p-6">
        <Result
          status="warning"
          title="插件页面已不可用"
          subTitle="该插件可能已经被禁用、卸载，或当前用户已失去访问该贡献的权限。"
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

      <Alert
        type="info"
        showIcon
        message="二期已完成命名空间路由解析"
        description="插件页面元数据、菜单和设置页路由已经接入官方后台；真正的插件模块挂载与宿主 SDK 合约将在第三期接入。"
      />

      <Card>
        <Descriptions column={1} items={descriptionItems} />
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