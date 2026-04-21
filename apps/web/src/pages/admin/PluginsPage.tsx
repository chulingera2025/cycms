import { Button, Popconfirm, Space, Table, Tag } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { usePluginAction, usePlugins } from '@/features/plugins/hooks';
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
  const { data: plugins, isLoading, refetch, isRefetching } = usePlugins();
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
        <Button onClick={() => refetch()} loading={isRefetching}>
          刷新
        </Button>
      </div>
      <Table<Plugin>
        rowKey="name"
        columns={columns}
        dataSource={plugins ?? []}
        loading={isLoading}
        pagination={false}
        scroll={{ x: 'max-content' }}
      />
    </div>
  );
}
