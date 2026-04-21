import { useState } from 'react';
import { Button, Popconfirm, Space, Table, Tag } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { Plus } from 'lucide-react';
import { ContentTypeForm } from '@/features/content-types/ContentTypeForm';
import { formValueToFieldDefinition } from '@/features/content-types/fieldType';
import {
  useContentTypes,
  useCreateContentType,
  useDeleteContentType,
  useUpdateContentType,
} from '@/features/content-types/hooks';
import type { ContentTypeFormValues } from '@/features/content-types/schema';
import { toast } from '@/lib/toast';
import type { ContentTypeDefinition } from '@/types';

export default function ContentTypesPage() {
  const { data: types = [], isLoading } = useContentTypes();
  const create = useCreateContentType();
  const update = useUpdateContentType();
  const del = useDeleteContentType();
  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<ContentTypeDefinition | null>(null);

  function openCreate() {
    setEditing(null);
    setOpen(true);
  }

  function openEdit(t: ContentTypeDefinition) {
    setEditing(t);
    setOpen(true);
  }

  async function handleSubmit(values: ContentTypeFormValues) {
    const fields = values.fields.map((field, index) => formValueToFieldDefinition(field, index));

    if (editing) {
      await update.mutateAsync({
        apiId: editing.api_id,
        input: {
          name: values.name,
          description: values.description,
          fields,
        },
      });
      toast.success(`已更新 ${values.name}`);
    } else {
      await create.mutateAsync({
        name: values.name,
        api_id: values.api_id,
        description: values.description,
        kind: values.kind,
        fields,
      });
      toast.success(`已创建 ${values.name}`);
    }
    setOpen(false);
  }

  const columns: ColumnsType<ContentTypeDefinition> = [
    {
      title: '名称',
      dataIndex: 'name',
      key: 'name',
      render: (v: string) => <span className="font-medium text-text">{v}</span>,
    },
    {
      title: 'API ID',
      dataIndex: 'api_id',
      key: 'api_id',
      render: (v: string) => <code className="font-mono text-xs text-text">{v}</code>,
    },
    {
      title: '类型',
      dataIndex: 'kind',
      key: 'kind',
      width: 120,
      render: (v: string) => (
        <Tag color={v === 'collection' ? 'blue' : 'purple'}>
          {v === 'collection' ? 'Collection' : 'Single'}
        </Tag>
      ),
    },
    {
      title: '字段数',
      dataIndex: 'fields',
      key: 'fields_count',
      width: 96,
      render: (fields: unknown[]) => fields.length,
    },
    {
      title: '更新时间',
      dataIndex: 'updated_at',
      key: 'updated_at',
      render: (v: string) => new Date(v).toLocaleString('zh-CN'),
      responsive: ['md'],
    },
    {
      title: '操作',
      key: 'actions',
      width: 180,
      render: (_, row) => (
        <Space size="small">
          <Button size="small" onClick={() => openEdit(row)}>
            编辑
          </Button>
          <Popconfirm
            title="删除内容类型"
            description={`删除 ${row.name}？关联的内容条目也会受影响。`}
            okButtonProps={{ danger: true }}
            okText="删除"
            cancelText="取消"
            onConfirm={async () => {
              await del.mutateAsync(row.api_id);
              toast.success(`已删除 ${row.name}`);
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
    <div className="p-6">
      <div className="mb-4 flex items-center justify-between gap-4">
        <div>
          <h1 className="m-0 text-xl font-semibold text-text">内容类型管理</h1>
          <p className="mt-1 text-sm text-text-muted">定义内容的字段结构</p>
        </div>
        <Button type="primary" icon={<Plus size={14} />} onClick={openCreate}>
          新建内容类型
        </Button>
      </div>

      <Table<ContentTypeDefinition>
        rowKey="id"
        columns={columns}
        dataSource={types}
        loading={isLoading}
        pagination={false}
        scroll={{ x: 'max-content' }}
      />

      <ContentTypeForm
        open={open}
        initial={editing}
        onClose={() => setOpen(false)}
        onSubmit={handleSubmit}
        loading={create.isPending || update.isPending}
      />
    </div>
  );
}
