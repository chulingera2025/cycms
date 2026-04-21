import { useState } from 'react';
import { Button, Empty, Form, Input, Popconfirm, Space, Table, Tabs } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { Pencil, Plus, Save, X } from 'lucide-react';
import {
  useDeleteSetting,
  useSetSetting,
  useSettings,
} from '@/features/settings/hooks';
import { toast } from '@/lib/toast';
import type { SettingsEntry } from '@/types';

const NAMESPACES = [
  { key: 'system', label: '系统' },
  { key: 'content', label: '内容' },
  { key: 'media', label: '媒体' },
  { key: 'auth', label: '认证' },
];

function formatValue(value: unknown): string {
  return typeof value === 'string' ? value : JSON.stringify(value, null, 2);
}

function parseValue(input: string): unknown {
  try {
    return JSON.parse(input);
  } catch {
    return input;
  }
}

function NamespacePanel({ namespace }: { namespace: string }) {
  const { data: entries = [], isLoading } = useSettings(namespace);
  const setMutation = useSetSetting();
  const delMutation = useDeleteSetting();
  const [editKey, setEditKey] = useState<string | null>(null);
  const [editValue, setEditValue] = useState('');
  const [newForm] = Form.useForm<{ key: string; value: string }>();

  function start(entry: SettingsEntry) {
    setEditKey(entry.key);
    setEditValue(formatValue(entry.value));
  }

  async function save(key: string) {
    await setMutation.mutateAsync({ namespace, key, value: parseValue(editValue) });
    toast.success('已保存');
    setEditKey(null);
  }

  async function add(values: { key: string; value?: string }) {
    await setMutation.mutateAsync({
      namespace,
      key: values.key,
      value: parseValue(values.value ?? ''),
    });
    toast.success(`已添加 ${values.key}`);
    newForm.resetFields();
  }

  const columns: ColumnsType<SettingsEntry> = [
    {
      title: '键',
      dataIndex: 'key',
      key: 'key',
      width: 220,
      render: (v: string) => <code className="font-mono text-sm text-text">{v}</code>,
    },
    {
      title: '值',
      dataIndex: 'value',
      key: 'value',
      render: (_: unknown, row) =>
        editKey === row.key ? (
          <Input.TextArea
            value={editValue}
            onChange={(e) => setEditValue(e.target.value)}
            autoSize={{ minRows: 2, maxRows: 8 }}
          />
        ) : (
          <pre className="m-0 max-h-32 overflow-auto rounded bg-surface-alt p-2 font-mono text-xs text-text">
            {formatValue(row.value)}
          </pre>
        ),
    },
    {
      title: '操作',
      key: 'actions',
      width: 200,
      render: (_: unknown, row) =>
        editKey === row.key ? (
          <Space size="small">
            <Button
              size="small"
              type="primary"
              icon={<Save size={12} />}
              loading={setMutation.isPending}
              onClick={() => save(row.key)}
            >
              保存
            </Button>
            <Button size="small" icon={<X size={12} />} onClick={() => setEditKey(null)}>
              取消
            </Button>
          </Space>
        ) : (
          <Space size="small">
            <Button
              size="small"
              icon={<Pencil size={12} />}
              onClick={() => start(row)}
            >
              编辑
            </Button>
            <Popconfirm
              title="删除配置"
              description={`删除 ${row.key}？`}
              okButtonProps={{ danger: true }}
              okText="删除"
              cancelText="取消"
              onConfirm={async () => {
                await delMutation.mutateAsync({ namespace, key: row.key });
                toast.success(`已删除 ${row.key}`);
              }}
            >
              <Button size="small" danger>
                删除
              </Button>
            </Popconfirm>
          </Space>
        ),
    },
  ];

  return (
    <div>
      <Table<SettingsEntry>
        rowKey="key"
        columns={columns}
        dataSource={entries}
        loading={isLoading}
        pagination={false}
        size="middle"
        locale={{
          emptyText: <Empty description={`${namespace} 命名空间暂无配置`} />,
        }}
      />

      <div className="mt-4 rounded border border-dashed border-border bg-surface-alt p-4">
        <div className="mb-2 font-medium text-text">添加配置</div>
        <Form form={newForm} layout="inline" onFinish={add}>
          <Form.Item name="key" rules={[{ required: true, message: '请输入键' }]}>
            <Input placeholder="键名（例：site_name）" style={{ width: 220 }} />
          </Form.Item>
          <Form.Item name="value">
            <Input
              placeholder='值（字符串或 JSON，例："CyCMS" 或 42）'
              style={{ width: 360 }}
            />
          </Form.Item>
          <Form.Item>
            <Button
              type="primary"
              htmlType="submit"
              icon={<Plus size={14} />}
              loading={setMutation.isPending}
            >
              添加
            </Button>
          </Form.Item>
        </Form>
      </div>
    </div>
  );
}

export default function SettingsPage() {
  const [active, setActive] = useState('system');

  return (
    <div className="p-6">
      <div className="mb-4">
        <h1 className="m-0 text-xl font-semibold text-text">系统设置</h1>
        <p className="mt-1 text-sm text-text-muted">
          按命名空间管理键值配置；值可为字符串或 JSON
        </p>
      </div>
      <Tabs
        activeKey={active}
        onChange={setActive}
        items={NAMESPACES.map((ns) => ({
          key: ns.key,
          label: ns.label,
          children: <NamespacePanel namespace={ns.key} />,
        }))}
      />
    </div>
  );
}
