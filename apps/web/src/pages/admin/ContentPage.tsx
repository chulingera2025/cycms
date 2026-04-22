import { useEffect, useMemo, useState } from 'react';
import { Button, Input, Popconfirm, Select, Space, Table, Tag } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { History, Plus } from 'lucide-react';
import { useSearchParams } from 'react-router-dom';
import { EntryEditor } from '@/features/content/EntryEditor';
import { RevisionDrawer } from '@/features/content/RevisionDrawer';
import {
  useContentList,
  useCreateEntry,
  useDeleteEntry,
  usePublishEntry,
  useUnpublishEntry,
  useUpdateEntry,
} from '@/features/content/hooks';
import { useContentTypes } from '@/features/content-types/hooks';
import { toast } from '@/lib/toast';
import type { ContentEntry, ContentStatus } from '@/types';

const STATUS_COLOR: Record<ContentStatus, string> = {
  draft: 'gold',
  published: 'green',
  archived: 'default',
};

const STATUS_LABEL: Record<ContentStatus, string> = {
  draft: '草稿',
  published: '已发布',
  archived: '已归档',
};

export default function ContentPage() {
  const { data: types = [], isLoading: typesLoading } = useContentTypes();
  const [searchParams, setSearchParams] = useSearchParams();
  const routeType = searchParams.get('type') ?? '';
  const [selectedType, setSelectedType] = useState<string>(routeType);
  const [statusFilter, setStatusFilter] = useState<ContentStatus | ''>('');
  const [slugSearch, setSlugSearch] = useState('');
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);

  const [editorOpen, setEditorOpen] = useState(false);
  const [editingEntry, setEditingEntry] = useState<ContentEntry | null>(null);

  const [revisionOpen, setRevisionOpen] = useState(false);
  const [revisionEntryId, setRevisionEntryId] = useState<string>('');

  useEffect(() => {
    if (routeType && routeType !== selectedType) {
      setSelectedType(routeType);
    }
  }, [routeType, selectedType]);

  const typeApiId = selectedType || types[0]?.api_id || '';
  const currentType = useMemo(
    () => types.find((t) => t.api_id === typeApiId) ?? null,
    [types, typeApiId],
  );

  const params = useMemo(() => {
    const p: Record<string, string> = {
      page: String(page),
      pageSize: String(pageSize),
    };
    if (statusFilter) p.status = statusFilter;
    if (slugSearch) p['filter[slug][contains]'] = slugSearch;
    return p;
  }, [page, pageSize, statusFilter, slugSearch]);

  const { data: entries, isLoading: entriesLoading } = useContentList(
    typeApiId,
    params,
  );

  const create = useCreateEntry(typeApiId);
  const update = useUpdateEntry(typeApiId);
  const del = useDeleteEntry(typeApiId);
  const publish = usePublishEntry(typeApiId);
  const unpublish = useUnpublishEntry(typeApiId);

  function openCreate() {
    setEditingEntry(null);
    setEditorOpen(true);
  }
  function openEdit(e: ContentEntry) {
    setEditingEntry(e);
    setEditorOpen(true);
  }
  function openRevisions(e: ContentEntry) {
    setRevisionEntryId(e.id);
    setRevisionOpen(true);
  }

  async function handleSubmit(payload: {
    data: Record<string, unknown>;
    slug?: string | null;
  }) {
    if (editingEntry) {
      await update.mutateAsync({
        id: editingEntry.id,
        input: { data: payload.data, slug: payload.slug },
      });
      toast.success('已更新');
    } else {
      await create.mutateAsync({
        data: payload.data,
        slug: payload.slug ?? undefined,
      });
      toast.success('已创建');
    }
    setEditorOpen(false);
  }

  const columns: ColumnsType<ContentEntry> = [
    {
      title: 'ID',
      dataIndex: 'id',
      key: 'id',
      width: 120,
      render: (v: string) => (
        <code className="font-mono text-xs text-text-muted" title={v}>
          {v.slice(0, 8)}
        </code>
      ),
    },
    {
      title: 'Slug',
      dataIndex: 'slug',
      key: 'slug',
      render: (v?: string) =>
        v ? <code className="font-mono text-xs">{v}</code> : <span className="text-text-muted">—</span>,
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 110,
      render: (s: ContentStatus) => (
        <Tag color={STATUS_COLOR[s]}>{STATUS_LABEL[s]}</Tag>
      ),
    },
    {
      title: '创建时间',
      dataIndex: 'created_at',
      key: 'created_at',
      render: (v: string) => new Date(v).toLocaleString('zh-CN'),
      responsive: ['md'],
    },
    {
      title: '操作',
      key: 'actions',
      width: 280,
      render: (_: unknown, row) => (
        <Space size="small" wrap>
          <Button size="small" onClick={() => openEdit(row)}>
            编辑
          </Button>
          <Button
            size="small"
            icon={<History size={12} />}
            onClick={() => openRevisions(row)}
          >
            版本
          </Button>
          {row.status === 'draft' && (
            <Popconfirm
              title="发布内容"
              description="发布后可被公开访问。"
              okText="发布"
              cancelText="取消"
              onConfirm={async () => {
                await publish.mutateAsync(row.id);
                toast.success('已发布');
              }}
            >
              <Button size="small" type="primary">
                发布
              </Button>
            </Popconfirm>
          )}
          {row.status === 'published' && (
            <Popconfirm
              title="撤回发布"
              description="撤回后将回到草稿状态。"
              okText="撤回"
              cancelText="取消"
              onConfirm={async () => {
                await unpublish.mutateAsync(row.id);
                toast.success('已撤回');
              }}
            >
              <Button size="small">撤回</Button>
            </Popconfirm>
          )}
          <Popconfirm
            title="删除内容"
            description="此操作不可撤销。"
            okButtonProps={{ danger: true }}
            okText="删除"
            cancelText="取消"
            onConfirm={async () => {
              await del.mutateAsync(row.id);
              toast.success('已删除');
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
      <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
        <h1 className="m-0 text-xl font-semibold text-text">内容管理</h1>
        <Space wrap size="small">
          <Select
            placeholder="选择内容类型"
            style={{ width: 200 }}
            value={typeApiId || undefined}
            loading={typesLoading}
            onChange={(v) => {
              setSelectedType(v);
              const next = new URLSearchParams(searchParams);
              next.set('type', v);
              setSearchParams(next);
              setPage(1);
            }}
            options={types.map((t) => ({ value: t.api_id, label: t.name }))}
          />
          <Select
            placeholder="全部状态"
            style={{ width: 140 }}
            value={statusFilter || undefined}
            allowClear
            onChange={(v) => {
              setStatusFilter((v as ContentStatus) ?? '');
              setPage(1);
            }}
            options={[
              { value: 'draft', label: '草稿' },
              { value: 'published', label: '已发布' },
              { value: 'archived', label: '已归档' },
            ]}
          />
          <Input.Search
            placeholder="搜索 slug"
            allowClear
            style={{ width: 200 }}
            onSearch={(v) => {
              setSlugSearch(v);
              setPage(1);
            }}
          />
          <Button
            type="primary"
            icon={<Plus size={14} />}
            onClick={openCreate}
            disabled={!currentType}
          >
            新建
          </Button>
        </Space>
      </div>

      <Table<ContentEntry>
        rowKey="id"
        columns={columns}
        dataSource={entries?.data ?? []}
        loading={entriesLoading}
        scroll={{ x: 'max-content' }}
        pagination={{
          current: page,
          pageSize,
          total: entries?.meta.total ?? 0,
          showSizeChanger: true,
          pageSizeOptions: [10, 20, 50, 100],
          onChange: (p, ps) => {
            setPage(p);
            setPageSize(ps);
          },
        }}
      />

      <EntryEditor
        open={editorOpen}
        contentType={currentType}
        initial={editingEntry}
        onClose={() => setEditorOpen(false)}
        onSubmit={handleSubmit}
        loading={create.isPending || update.isPending}
      />

      <RevisionDrawer
        open={revisionOpen}
        typeApiId={typeApiId}
        entryId={revisionEntryId}
        onClose={() => setRevisionOpen(false)}
      />
    </div>
  );
}
