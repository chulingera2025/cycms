import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import {
  Alert,
  Button,
  Descriptions,
  Drawer,
  Empty,
  List,
  Popconfirm,
  Space,
  Table,
  Tag,
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { usePluginAction, usePlugins } from '@/features/plugins/hooks';
import { adminExtensionsApi } from '@/lib/api/admin-extensions';
import { qk } from '@/lib/query-keys';
import { toast } from '@/lib/toast';
import type { Plugin, PluginStatus } from '@/types';

const STATUS_COLOR: Record<PluginStatus, string> = {
  discovered: 'default',
  installed: 'blue',
  enabled: 'green',
  disabled: 'gold',
  error: 'red',
};

const STATUS_LABEL: Record<PluginStatus, string> = {
  discovered: '已发现',
  installed: '已安装',
  enabled: '已启用',
  disabled: '已禁用',
  error: '异常',
};

export default function PluginsPage() {
  const [diagnosticsOpen, setDiagnosticsOpen] = useState(false);
  const { data: plugins, isLoading, refetch, isRefetching } = usePlugins();
  const {
    data: diagnostics,
    isLoading: diagnosticsLoading,
    refetch: refetchDiagnostics,
    isRefetching: diagnosticsRefetching,
  } = useQuery({
    queryKey: qk.adminExtensions.diagnostics,
    queryFn: () => adminExtensionsApi.diagnostics(),
  });
  const install = usePluginAction('install');
  const enable = usePluginAction('enable');
  const disable = usePluginAction('disable');
  const uninstall = usePluginAction('uninstall');

  const columns: ColumnsType<Plugin> = [
    {
      title: '名称',
      dataIndex: 'name',
      key: 'name',
      render: (name: string, row) => (
        <div className="flex flex-col">
          <span className="font-medium text-text">{name}</span>
          {row.description && (
            <span className="text-xs text-text-muted">{row.description}</span>
          )}
        </div>
      ),
    },
    { title: '版本', dataIndex: 'version', key: 'version', width: 120 },
    { title: '运行时', dataIndex: 'runtime', key: 'runtime', width: 120 },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 120,
      render: (s: PluginStatus) => <Tag color={STATUS_COLOR[s]}>{STATUS_LABEL[s]}</Tag>,
    },
    {
      title: '操作',
      key: 'actions',
      width: 260,
      render: (_, row) => (
        <Space size="small" wrap>
          {row.status === 'discovered' && (
            <Button
              size="small"
              type="primary"
              loading={install.isPending}
              onClick={async () => {
                await install.mutateAsync(row.name);
                toast.success(`已安装 ${row.name}`);
              }}
            >
              安装
            </Button>
          )}
          {(row.status === 'installed' || row.status === 'disabled') && (
            <Button
              size="small"
              type="primary"
              loading={enable.isPending}
              onClick={async () => {
                await enable.mutateAsync(row.name);
                toast.success(`已启用 ${row.name}`);
              }}
            >
              启用
            </Button>
          )}
          {row.status === 'enabled' && (
            <Button
              size="small"
              loading={disable.isPending}
              onClick={async () => {
                await disable.mutateAsync(row.name);
                toast.success(`已禁用 ${row.name}`);
              }}
            >
              禁用
            </Button>
          )}
          {(row.status === 'installed' ||
            row.status === 'disabled' ||
            row.status === 'error') && (
            <Popconfirm
              title="卸载插件"
              description={`确定卸载 ${row.name}？该操作不可撤销。`}
              okButtonProps={{ danger: true }}
              okText="卸载"
              cancelText="取消"
              onConfirm={async () => {
                await uninstall.mutateAsync(row.name);
                toast.success(`已卸载 ${row.name}`);
              }}
            >
              <Button size="small" danger loading={uninstall.isPending}>
                卸载
              </Button>
            </Popconfirm>
          )}
        </Space>
      ),
    },
  ];

  return (
    <div className="p-6">
      <div className="mb-4 flex items-center justify-between gap-4">
        <div>
          <h1 className="m-0 text-xl font-semibold text-text">插件管理</h1>
          <p className="mt-1 text-sm text-text-muted">
            将插件放入 <code className="font-mono">plugins/</code> 目录后点击刷新发现
          </p>
        </div>
        <Space>
          <Button onClick={() => setDiagnosticsOpen(true)}>扩展诊断</Button>
          <Button onClick={() => refetch()} loading={isRefetching}>
            刷新
          </Button>
        </Space>
      </div>
      <Table<Plugin>
        rowKey="name"
        columns={columns}
        dataSource={plugins ?? []}
        loading={isLoading}
        pagination={false}
        scroll={{ x: 'max-content' }}
      />
      <Drawer
        open={diagnosticsOpen}
        onClose={() => setDiagnosticsOpen(false)}
        title="扩展诊断与最近事件"
        width={760}
        extra={
          <Button onClick={() => refetchDiagnostics()} loading={diagnosticsRefetching}>
            刷新诊断
          </Button>
        }
      >
        {diagnosticsLoading && <Alert type="info" showIcon message="正在加载扩展诊断信息" />}
        {diagnostics && (
          <div className="space-y-6">
            <Descriptions
              column={1}
              items={[
                { key: 'revision', label: '当前 revision', children: diagnostics.revision },
                {
                  key: 'csp',
                  label: 'CSP 模式',
                  children: diagnostics.security.cspEnabled
                    ? diagnostics.security.cspReportOnly
                      ? `${diagnostics.security.cspHeaderName}（report-only）`
                      : diagnostics.security.cspHeaderName
                    : '未启用',
                },
                {
                  key: 'policy',
                  label: 'CSP 策略',
                  children: (
                    <pre className="m-0 whitespace-pre-wrap break-all rounded bg-surface-alt p-3 text-xs text-text-muted">
                      {diagnostics.security.cspPolicy || '未配置'}
                    </pre>
                  ),
                },
              ]}
            />

            <section className="space-y-3">
              <div>
                <h2 className="m-0 text-base font-semibold text-text">Bootstrap 诊断</h2>
                <p className="mt-1 text-sm text-text-muted">
                  当前展示的是后端根据已安装插件 frontend runtime state 计算出的诊断结果。
                </p>
              </div>
              {diagnostics.diagnostics.length > 0 ? (
                <List
                  dataSource={diagnostics.diagnostics}
                  renderItem={(item) => (
                    <List.Item>
                      <Alert
                        type={item.severity === 'error' ? 'error' : item.severity === 'warning' ? 'warning' : 'info'}
                        showIcon
                        message={`${item.pluginName}@${item.pluginVersion} · ${item.code}`}
                        description={item.message}
                      />
                    </List.Item>
                  )}
                />
              ) : (
                <Empty description="当前没有 frontend diagnostics" image={Empty.PRESENTED_IMAGE_SIMPLE} />
              )}
            </section>

            <section className="space-y-3">
              <div>
                <h2 className="m-0 text-base font-semibold text-text">最近事件</h2>
                <p className="mt-1 text-sm text-text-muted">
                  包含宿主上报的模块 load/mount/unmount、route resolution、插件动作以及 CSP report。
                </p>
              </div>
              {diagnostics.recentEvents.length > 0 ? (
                <List
                  itemLayout="vertical"
                  dataSource={diagnostics.recentEvents}
                  renderItem={(item) => (
                    <List.Item>
                      <div className="space-y-2 rounded border border-border bg-surface p-3">
                        <div className="flex flex-wrap items-center gap-2">
                          <Tag color={item.level === 'error' ? 'red' : item.level === 'warning' ? 'gold' : 'blue'}>
                            {item.level}
                          </Tag>
                          <Tag>{item.source}</Tag>
                          <Tag>{item.eventName}</Tag>
                          {item.pluginName && <Tag color="purple">{item.pluginName}</Tag>}
                        </div>
                        <div className="text-sm text-text">{item.message}</div>
                        <div className="text-xs text-text-muted">
                          {item.recordedAt}
                          {item.fullPath ? ` · ${item.fullPath}` : ''}
                          {item.requestId ? ` · request ${item.requestId}` : ''}
                        </div>
                        {item.detail !== undefined && item.detail !== null ? (
                          <pre className="m-0 whitespace-pre-wrap break-all rounded bg-surface-alt p-3 text-xs text-text-muted">
                            {JSON.stringify(item.detail, null, 2)}
                          </pre>
                        ) : null}
                      </div>
                    </List.Item>
                  )}
                />
              ) : (
                <Empty description="当前还没有扩展事件" image={Empty.PRESENTED_IMAGE_SIMPLE} />
              )}
            </section>
          </div>
        )}
      </Drawer>
    </div>
  );
}
