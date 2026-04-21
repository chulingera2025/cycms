import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Alert,
  Button,
  Empty,
  Form,
  Input,
  InputNumber,
  Popconfirm,
  Select,
  Space,
  Switch,
  Table,
} from 'antd';
import type { Rule } from 'antd/es/form';
import type { ColumnsType } from 'antd/es/table';
import { ExternalLink, Pencil, Plus, Save, X } from 'lucide-react';
import {
  useDeleteSetting,
  useSetSetting,
  useSettings,
} from '@/features/settings/hooks';
import { toast } from '@/lib/toast';
import type {
  BootstrapSettingsContribution,
  JsonSchemaNode,
  SettingsEntry,
} from '@/types';

type SupportedFieldKind =
  | 'string'
  | 'number'
  | 'integer'
  | 'boolean'
  | 'enum'
  | 'json';

interface SchemaFieldDefinition {
  key: string;
  label: string;
  description?: string;
  required: boolean;
  kind: SupportedFieldKind;
  enumOptions: Array<{ label: string; value: string }>;
  schema: JsonSchemaNode;
  defaultValue: unknown;
}

function formatValue(value: unknown): string {
  return typeof value === 'string' ? value : JSON.stringify(value, null, 2);
}

function parseLooseValue(input: string): unknown {
  try {
    return JSON.parse(input);
  } catch {
    return input;
  }
}

function getSchemaType(schema: JsonSchemaNode): SupportedFieldKind {
  if (schema.enum?.length) {
    return 'enum';
  }

  const schemaType = Array.isArray(schema.type) ? schema.type[0] : schema.type;
  switch (schemaType) {
    case 'boolean':
      return 'boolean';
    case 'number':
      return 'number';
    case 'integer':
      return 'integer';
    case 'string':
      return 'string';
    default:
      return 'json';
  }
}

function collectSchemaFields(schema: JsonSchemaNode): SchemaFieldDefinition[] {
  const properties = schema.properties ?? {};
  const required = new Set(schema.required ?? []);

  return Object.entries(properties)
    .map(([key, definition]) => ({
      key,
      label: definition.title ?? key,
      description: definition.description,
      required: required.has(key),
      kind: getSchemaType(definition),
      enumOptions: (definition.enum ?? []).map((value) => ({
        label: String(value),
        value: JSON.stringify(value),
      })),
      schema: definition,
      defaultValue: definition.default,
    }))
    .sort((left, right) => left.label.localeCompare(right.label, 'zh-CN'));
}

function defaultJsonValue(schema: JsonSchemaNode) {
  const schemaType = Array.isArray(schema.type) ? schema.type[0] : schema.type;
  if (schemaType === 'array') {
    return [];
  }
  if (schemaType === 'object') {
    return {};
  }
  return null;
}

function toFormValue(field: SchemaFieldDefinition, value: unknown) {
  const resolved = value ?? field.defaultValue;
  switch (field.kind) {
    case 'boolean':
      return typeof resolved === 'boolean' ? resolved : false;
    case 'number':
    case 'integer':
      return typeof resolved === 'number' ? resolved : undefined;
    case 'enum':
      return resolved === undefined ? undefined : JSON.stringify(resolved);
    case 'json':
      return JSON.stringify(resolved ?? defaultJsonValue(field.schema), null, 2);
    case 'string':
    default:
      return typeof resolved === 'string' ? resolved : resolved == null ? '' : String(resolved);
  }
}

function fromFormValue(field: SchemaFieldDefinition, value: unknown) {
  const resolved = value === undefined ? field.defaultValue : value;
  switch (field.kind) {
    case 'boolean':
      return Boolean(resolved);
    case 'number':
    case 'integer':
      return resolved == null || resolved === '' ? null : Number(resolved);
    case 'enum':
      return typeof resolved === 'string' ? JSON.parse(resolved) : resolved;
    case 'json':
      return typeof resolved === 'string' ? JSON.parse(resolved) : resolved;
    case 'string':
    default:
      return resolved == null ? '' : resolved;
  }
}

function valuesEqual(left: unknown, right: unknown) {
  return JSON.stringify(left) === JSON.stringify(right);
}

export function RawNamespacePanel({ namespace }: { namespace: string }) {
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
    await setMutation.mutateAsync({ namespace, key, value: parseLooseValue(editValue) });
    toast.success('已保存');
    setEditKey(null);
  }

  async function add(values: { key: string; value?: string }) {
    await setMutation.mutateAsync({
      namespace,
      key: values.key,
      value: parseLooseValue(values.value ?? ''),
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
      render: (value: string) => <code className="font-mono text-sm text-text">{value}</code>,
    },
    {
      title: '值',
      dataIndex: 'value',
      key: 'value',
      render: (_value: unknown, row) =>
        editKey === row.key ? (
          <Input.TextArea
            value={editValue}
            onChange={(event) => setEditValue(event.target.value)}
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
      width: 220,
      render: (_value: unknown, row) =>
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
            <Button size="small" icon={<Pencil size={12} />} onClick={() => start(row)}>
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

export function SchemaNamespacePanel({
  namespace,
  schema,
}: {
  namespace: string;
  schema: JsonSchemaNode;
}) {
  const { data: entries = [], isLoading } = useSettings(namespace);
  const setMutation = useSetSetting();
  const [form] = Form.useForm<Record<string, unknown>>();
  const fields = useMemo(() => collectSchemaFields(schema), [schema]);

  const initialValues = useMemo(() => {
    const entryMap = new Map(entries.map((entry) => [entry.key, entry.value]));
    return Object.fromEntries(
      fields.map((field) => [field.key, toFormValue(field, entryMap.get(field.key))]),
    );
  }, [entries, fields]);

  useEffect(() => {
    form.setFieldsValue(initialValues);
  }, [form, initialValues]);

  async function handleSubmit(values: Record<string, unknown>) {
    const currentValues = new Map(entries.map((entry) => [entry.key, entry.value]));
    for (const field of fields) {
      const nextValue = fromFormValue(field, values[field.key]);
      if (valuesEqual(nextValue, currentValues.get(field.key))) {
        continue;
      }
      await setMutation.mutateAsync({ namespace, key: field.key, value: nextValue });
    }
    toast.success('已保存命名空间配置');
  }

  if (!fields.length) {
    return <Alert type="info" showIcon message="当前 schema 未声明可编辑字段" />;
  }

  return (
    <div className="space-y-4">
      {(schema.title || schema.description) && (
        <Alert
          type="info"
          showIcon
          message={schema.title ?? `${namespace} 设置`}
          description={schema.description as string | undefined}
        />
      )}
      <Form form={form} layout="vertical" onFinish={handleSubmit}>
        {fields.map((field) => {
          const rules: Rule[] = field.required
            ? [{ required: true, message: `请输入${field.label}` }]
            : [];
          if (field.kind === 'json') {
            rules.push({
              validator: async (_rule: Rule, value?: string) => {
                if (typeof value !== 'string') {
                  return;
                }
                try {
                  JSON.parse(value);
                } catch {
                  throw new Error(`${field.label} 需要是合法 JSON`);
                }
              },
            });
          }

          return (
            <Form.Item
              key={field.key}
              name={field.key}
              label={field.label}
              rules={rules}
              valuePropName={field.kind === 'boolean' ? 'checked' : 'value'}
              extra={field.description}
            >
              {field.kind === 'boolean' ? (
                <Switch />
              ) : field.kind === 'number' || field.kind === 'integer' ? (
                <InputNumber style={{ width: '100%' }} precision={field.kind === 'integer' ? 0 : undefined} />
              ) : field.kind === 'enum' ? (
                <Select
                  allowClear={!field.required}
                  options={field.enumOptions}
                  placeholder={`请选择${field.label}`}
                />
              ) : field.kind === 'json' ? (
                <Input.TextArea autoSize={{ minRows: 4, maxRows: 12 }} />
              ) : (
                <Input />
              )}
            </Form.Item>
          );
        })}

        <Form.Item className="mb-0">
          <Space>
            <Button type="primary" htmlType="submit" loading={setMutation.isPending || isLoading}>
              保存命名空间配置
            </Button>
            <Button onClick={() => form.resetFields()}>重置</Button>
          </Space>
        </Form.Item>
      </Form>
    </div>
  );
}

export function CustomSettingsPanel({
  namespace,
  pluginName,
  contribution,
}: {
  namespace: string;
  pluginName: string;
  contribution: BootstrapSettingsContribution;
}) {
  const navigate = useNavigate();

  return (
    <div className="space-y-4">
      <Alert
        type="info"
        showIcon
        message="插件自定义设置页已注册"
        description="当前宿主已解析到插件设置页路由。正式的插件模块挂载将在第三期接入；在此之前，你仍可在下方使用宿主原生表格管理该命名空间。"
        action={
          contribution.customPage ? (
            <Button
              type="primary"
              icon={<ExternalLink size={14} />}
              onClick={() => navigate(contribution.customPage!.fullPath)}
            >
              打开插件设置页
            </Button>
          ) : undefined
        }
      />
      <RawNamespacePanel namespace={namespace} />
      <div className="text-xs text-text-muted">当前插件：{pluginName}</div>
    </div>
  );
}