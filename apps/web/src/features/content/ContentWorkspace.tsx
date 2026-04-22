import { useEffect, useMemo, useState } from 'react';
import { Button, Input, Popconfirm, Select, Space, Table, Tag, Typography } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { History, Plus } from 'lucide-react';
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

export interface ContentWorkspaceProps {
  pageTitle: string;
  pageDescription?: string;
  fixedTypeApiId?: string;
  selectedTypeApiId?: string;
  onSelectedTypeChange?: (typeApiId: string) => void;
  createLabel?: string;
  autoOpenCreate?: boolean;
  defaultStatusFilter?: ContentStatus | '';
  showTypeSelector?: boolean;
  showStatusFilter?: boolean;
  showSlugSearch?: boolean;
  slugSearchPlaceholder?: string;
}

export function ContentWorkspace({
  pageTitle,
  pageDescription,
  fixedTypeApiId,
  selectedTypeApiId,
  onSelectedTypeChange,
  createLabel = '新建',
  autoOpenCreate = false,
  defaultStatusFilter = '',
  showTypeSelector = true,
  showStatusFilter = true,
  showSlugSearch = true,
  slugSearchPlaceholder = '搜索 slug',
}: ContentWorkspaceProps) {
  const { data: types = [], isLoading: typesLoading } = useContentTypes();
  const [selectedType, setSelectedType] = useState<string>(selectedTypeApiId ?? '');
  const [statusFilter, setStatusFilter] = useState<ContentStatus | ''>(defaultStatusFilter);
  const [slugSearch, setSlugSearch] = useState('');
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);

  const [editorOpen, setEditorOpen] = useState(false);
  const [editingEntry, setEditingEntry] = useState<ContentEntry | null>(null);
  const [didAutoOpenCreate, setDidAutoOpenCreate] = useState(false);

  const [revisionOpen, setRevisionOpen] = useState(false);
  const [revisionEntryId, setRevisionEntryId] = useState<string>('');

  useEffect(() => {
    if (selectedTypeApiId) {
      setSelectedType(selectedTypeApiId);
    }
  }, [selectedTypeApiId]);

  const typeApiId = fixedTypeApiId || selectedType || types[0]?.api_id || '';
  const currentType = useMemo(
    () => types.find((t) => t.api_id === typeApiId) ?? null,
    [types, typeApiId],
  );

  useEffect(() => {
    if (autoOpenCreate && currentType && !didAutoOpenCreate) {
      setEditingEntry(null);
      setEditorOpen(true);
      setDidAutoOpenCreate(true);
    }
  }, [autoOpenCreate, currentType, didAutoOpenCreate]);

  const params = useMemo(() => {
    const next: Record<string, string> = {
      page: String(page),
      pageSize: String(pageSize),
    };
    if (statusFilter) next.status = statusFilter;
    if (slugSearch) next['filter[slug][contains]'] = slugSearch;
    return next;
  }, [page, pageSize, slugSearch, statusFilter]);

  const { data: entries, isLoading: entriesLoading } = useContentList(typeApiId, params);
  const isSingleType = currentType?.kind === 'single';
  const primarySingleEntry = isSingleType ? (entries?.data[0] ?? null) : null;
  const isSingleTypePending = isSingleType && entriesLoading && !entries;

  const create = useCreateEntry(typeApiId);
  const update = useUpdateEntry(typeApiId);
  const del = useDeleteEntry(typeApiId);
  const publish = usePublishEntry(typeApiId);
  const unpublish = useUnpublishEntry(typeApiId);

  function openCreate() {
    setEditingEntry(null);
    setEditorOpen(true);
  }

  function openPrimaryAction() {
    if (primarySingleEntry) {
      openEdit(primarySingleEntry);
      return;
    }
    openCreate();
  }

  function openEdit(entry: ContentEntry) {
    setEditingEntry(entry);
    setEditorOpen(true);
  }

  function openRevisions(entry: ContentEntry) {
    setRevisionEntryId(entry.id);
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
      render: (value: string) => (
        <code className="font-mono text-xs text-text-muted" title={value}>
          {value.slice(0, 8)}
        </code>
      ),
    },
    {
      title: 'Slug',
      dataIndex: 'slug',
      key: 'slug',
      render: (value?: string) =>
        value ? <code className="font-mono text-xs">{value}</code> : <span className="text-text-muted">—</span>,
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 110,
      render: (status: ContentStatus) => <Tag color={STATUS_COLOR[status]}>{STATUS_LABEL[status]}</Tag>,
    },
    {
      title: '创建时间',
      dataIndex: 'created_at',
      key: 'created_at',
      render: (value: string) => new Date(value).toLocaleString('zh-CN'),
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
        <div>
          <h1 className="m-0 text-xl font-semibold text-text">{pageTitle}</h1>
          {pageDescription && (
            <Typography.Paragraph type="secondary" style={{ marginTop: 8, marginBottom: 0 }}>
              {pageDescription}
            </Typography.Paragraph>
          )}
        </div>

        <Space wrap size="small">
          {showTypeSelector && !fixedTypeApiId && (
            <Select
              placeholder="选择内容类型"
              style={{ width: 200 }}
              value={typeApiId || undefined}
              loading={typesLoading}
              onChange={(value) => {
                setSelectedType(value);
                onSelectedTypeChange?.(value);
                setPage(1);
              }}
              options={types.map((type) => ({ value: type.api_id, label: type.name }))}
            />
          )}

          {showStatusFilter && (
            <Select
              placeholder="全部状态"
              style={{ width: 140 }}
              value={statusFilter || undefined}
              allowClear
              onChange={(value) => {
                setStatusFilter((value as ContentStatus) ?? '');
                setPage(1);
              }}
              options={[
                { value: 'draft', label: '草稿' },
                { value: 'published', label: '已发布' },
                { value: 'archived', label: '已归档' },
              ]}
            />
          )}

          {showSlugSearch && (
            <Input.Search
              placeholder={slugSearchPlaceholder}
              allowClear
              style={{ width: 220 }}
              onSearch={(value) => {
                setSlugSearch(value);
                setPage(1);
              }}
            />
          )}

          <Button
            type="primary"
            icon={<Plus size={14} />}
            onClick={openPrimaryAction}
            loading={isSingleTypePending}
            disabled={!currentType || isSingleTypePending}
          >
            {primarySingleEntry ? '编辑当前内容' : createLabel}
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
          onChange: (nextPage, nextPageSize) => {
            setPage(nextPage);
            setPageSize(nextPageSize);
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