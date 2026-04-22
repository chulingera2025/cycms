import { useMemo, useState } from 'react';
import {
  Button,
  Card,
  Empty,
  Image,
  Pagination,
  Popconfirm,
  Segmented,
  Select,
  Space,
  Table,
  Tag,
  Upload,
  type UploadProps,
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { Copy, Trash2, UploadCloud } from 'lucide-react';
import { useDeleteMedia, useMediaList, useUploadMedia } from '@/features/media/hooks';
import { toast } from '@/lib/toast';
import { formatBytes, resolveMediaUrl } from '@/utils/format';
import type { MediaAsset } from '@/types';

const MIME_OPTIONS = [
  { value: 'image/jpeg', label: 'JPEG' },
  { value: 'image/png', label: 'PNG' },
  { value: 'image/webp', label: 'WebP' },
  { value: 'image/gif', label: 'GIF' },
  { value: 'application/pdf', label: 'PDF' },
  { value: 'video/mp4', label: 'MP4' },
];

export interface MediaWorkspaceProps {
  pageTitle?: string;
  pageDescription?: string;
}

export function MediaWorkspace({
  pageTitle = '媒体管理',
  pageDescription,
}: MediaWorkspaceProps) {
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  const [mime, setMime] = useState<string>('');

  const params = useMemo(() => {
    const next: Record<string, string> = { page: String(page), pageSize: String(pageSize) };
    if (mime) {
      next.mime_type = mime;
    }
    return next;
  }, [mime, page, pageSize]);

  const { data, isLoading } = useMediaList(params);
  const upload = useUploadMedia();
  const del = useDeleteMedia();

  const uploadProps: UploadProps = {
    multiple: true,
    showUploadList: false,
    accept: 'image/*,application/pdf,video/mp4',
    customRequest: async ({ file, onSuccess, onError }) => {
      try {
        await upload.mutateAsync(file as File);
        toast.success(`已上传 ${(file as File).name}`);
        onSuccess?.({});
      } catch (error) {
        toast.error(`${(file as File).name} 上传失败`);
        onError?.(error as Error);
      }
    },
  };

  async function handleCopy(url: string) {
    try {
      await navigator.clipboard.writeText(url);
      toast.success('链接已复制');
    } catch {
      toast.error('复制失败');
    }
  }

  const listColumns: ColumnsType<MediaAsset> = [
    {
      title: '预览',
      key: 'preview',
      width: 72,
      render: (_: unknown, row) =>
        row.mime_type.startsWith('image/') ? (
          <Image
            src={resolveMediaUrl(row.storage_path)}
            alt={row.original_filename}
            width={56}
            height={56}
            style={{ objectFit: 'cover', borderRadius: 4 }}
          />
        ) : (
          <div className="grid h-14 w-14 place-items-center rounded bg-surface-alt font-mono text-xs text-text-secondary">
            {row.mime_type.split('/')[1]?.toUpperCase().slice(0, 4) ?? 'FILE'}
          </div>
        ),
    },
    {
      title: '文件名',
      dataIndex: 'original_filename',
      key: 'original_filename',
      ellipsis: true,
    },
    {
      title: '类型',
      dataIndex: 'mime_type',
      key: 'mime_type',
      width: 140,
      render: (value: string) => <Tag>{value}</Tag>,
    },
    {
      title: '大小',
      dataIndex: 'size',
      key: 'size',
      width: 100,
      render: (value: number) => formatBytes(value),
    },
    {
      title: '上传时间',
      dataIndex: 'created_at',
      key: 'created_at',
      render: (value: string) => new Date(value).toLocaleString('zh-CN'),
      responsive: ['md'],
    },
    {
      title: '操作',
      key: 'actions',
      width: 200,
      render: (_: unknown, row) => (
        <Space size="small">
          <Button
            size="small"
            icon={<Copy size={12} />}
            onClick={() => handleCopy(window.location.origin + resolveMediaUrl(row.storage_path))}
          >
            链接
          </Button>
          <Popconfirm
            title="删除媒体"
            description={`删除 ${row.original_filename}？`}
            okButtonProps={{ danger: true }}
            okText="删除"
            cancelText="取消"
            onConfirm={async () => {
              await del.mutateAsync(row.id);
              toast.success('已删除');
            }}
          >
            <Button size="small" danger icon={<Trash2 size={12} />} />
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
          {pageDescription && <p className="mt-2 text-sm text-text-muted">{pageDescription}</p>}
        </div>
        <Space size="small" wrap>
          <Select
            placeholder="全部类型"
            style={{ width: 140 }}
            value={mime || undefined}
            allowClear
            options={MIME_OPTIONS}
            onChange={(value) => {
              setMime(value ?? '');
              setPage(1);
            }}
          />
          <Segmented
            value={viewMode}
            onChange={(value) => setViewMode(value as typeof viewMode)}
            options={[
              { value: 'grid', label: '网格' },
              { value: 'list', label: '列表' },
            ]}
          />
        </Space>
      </div>

      <Upload.Dragger {...uploadProps} className="mb-4">
        <p className="ant-upload-drag-icon">
          <UploadCloud size={28} style={{ display: 'inline-block' }} />
        </p>
        <p className="ant-upload-text">点击或拖拽文件到此区域上传</p>
        <p className="ant-upload-hint">支持多文件并行；单文件 ≤ 10MB</p>
      </Upload.Dragger>

      {viewMode === 'grid' ? (
        isLoading ? null : (data?.data ?? []).length === 0 ? (
          <Empty description="暂无媒体" />
        ) : (
          <>
            <Image.PreviewGroup>
              <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
                {data?.data.map((asset) => (
                  <Card key={asset.id} styles={{ body: { padding: 12 } }}>
                    {asset.mime_type.startsWith('image/') ? (
                      <Image
                        src={resolveMediaUrl(asset.storage_path)}
                        alt={asset.original_filename}
                        style={{
                          objectFit: 'cover',
                          aspectRatio: '1 / 1',
                          borderRadius: 4,
                          width: '100%',
                        }}
                      />
                    ) : (
                      <div className="grid aspect-square place-items-center rounded bg-surface-alt font-mono text-sm text-text-secondary">
                        {asset.mime_type.split('/')[1]?.toUpperCase().slice(0, 4) ?? 'FILE'}
                      </div>
                    )}
                    <div
                      className="mt-2 truncate text-xs font-medium text-text"
                      title={asset.original_filename}
                    >
                      {asset.original_filename}
                    </div>
                    <div className="text-xs text-text-muted">{formatBytes(asset.size)}</div>
                    <Space size={4} className="mt-2">
                      <Button
                        size="small"
                        icon={<Copy size={12} />}
                        onClick={() => handleCopy(window.location.origin + resolveMediaUrl(asset.storage_path))}
                      />
                      <Popconfirm
                        title="删除媒体"
                        description={`删除 ${asset.original_filename}？`}
                        okButtonProps={{ danger: true }}
                        okText="删除"
                        cancelText="取消"
                        onConfirm={async () => {
                          await del.mutateAsync(asset.id);
                          toast.success('已删除');
                        }}
                      >
                        <Button size="small" danger icon={<Trash2 size={12} />} />
                      </Popconfirm>
                    </Space>
                  </Card>
                ))}
              </div>
            </Image.PreviewGroup>
            {data && data.page_count > 1 && (
              <div className="mt-6 flex justify-center">
                <Pagination
                  current={page}
                  pageSize={pageSize}
                  total={data.total}
                  showSizeChanger
                  pageSizeOptions={[10, 20, 50, 100]}
                  onChange={(nextPage, nextPageSize) => {
                    setPage(nextPage);
                    setPageSize(nextPageSize);
                  }}
                />
              </div>
            )}
          </>
        )
      ) : (
        <Table<MediaAsset>
          rowKey="id"
          columns={listColumns}
          dataSource={data?.data ?? []}
          loading={isLoading}
          scroll={{ x: 'max-content' }}
          pagination={{
            current: page,
            pageSize,
            total: data?.total ?? 0,
            showSizeChanger: true,
            onChange: (nextPage, nextPageSize) => {
              setPage(nextPage);
              setPageSize(nextPageSize);
            },
          }}
        />
      )}
    </div>
  );
}