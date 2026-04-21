import { useMemo, useState } from 'react';
import {
  Button,
  Empty,
  Image,
  Modal,
  Pagination,
  Segmented,
  Select,
  Space,
  Spin,
} from 'antd';
import clsx from 'clsx';
import { useMediaList } from './hooks';
import { formatBytes, resolveMediaUrl } from '@/utils/format';
import type { MediaAsset } from '@/types';

interface Props {
  open: boolean;
  multiple?: boolean;
  onClose: () => void;
  onSelect: (ids: string[], assets: MediaAsset[]) => void;
  initialSelected?: string[];
  accept?: 'image' | 'any';
}

const MIME_OPTIONS = [
  { value: '', label: '全部' },
  { value: 'image/jpeg', label: 'JPEG' },
  { value: 'image/png', label: 'PNG' },
  { value: 'image/webp', label: 'WebP' },
  { value: 'image/gif', label: 'GIF' },
  { value: 'application/pdf', label: 'PDF' },
  { value: 'video/mp4', label: 'MP4' },
];

export function MediaPicker({
  open,
  multiple = false,
  onClose,
  onSelect,
  initialSelected = [],
  accept = 'any',
}: Props) {
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(12);
  const [mime, setMime] = useState(accept === 'image' ? 'image/png' : '');
  const [selected, setSelected] = useState<MediaAsset[]>([]);

  const params = useMemo<Record<string, string>>(() => {
    const p: Record<string, string> = { page: String(page), pageSize: String(pageSize) };
    if (mime) p.mime_type = mime;
    return p;
  }, [page, pageSize, mime]);

  const { data, isLoading } = useMediaList(params);

  const mimeOptions = accept === 'image'
    ? MIME_OPTIONS.filter((o) => o.value === '' || o.value.startsWith('image/'))
    : MIME_OPTIONS;

  function toggle(asset: MediaAsset) {
    setSelected((prev) => {
      if (multiple) {
        return prev.some((a) => a.id === asset.id)
          ? prev.filter((a) => a.id !== asset.id)
          : [...prev, asset];
      }
      return prev[0]?.id === asset.id ? [] : [asset];
    });
  }

  function reset() {
    setSelected([]);
    setPage(1);
  }

  function handleOk() {
    onSelect(
      selected.map((a) => a.id),
      selected,
    );
    reset();
    onClose();
  }

  function handleCancel() {
    reset();
    onClose();
  }

  const isPicked = (asset: MediaAsset) =>
    selected.some((a) => a.id === asset.id) || initialSelected.includes(asset.id);

  return (
    <Modal
      open={open}
      onCancel={handleCancel}
      onOk={handleOk}
      title="选择媒体"
      width={820}
      okText={selected.length > 0 ? `确定 (${selected.length})` : '确定'}
      okButtonProps={{ disabled: selected.length === 0 }}
      destroyOnClose
    >
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <Select
          value={mime}
          style={{ width: 160 }}
          options={mimeOptions}
          onChange={(v) => {
            setMime(v);
            setPage(1);
          }}
        />
        <Segmented
          options={[
            { value: 12, label: '12 / 页' },
            { value: 24, label: '24 / 页' },
          ]}
          value={pageSize}
          onChange={(v) => {
            setPageSize(v as number);
            setPage(1);
          }}
        />
      </div>

      <Spin spinning={isLoading}>
        {(data?.data ?? []).length === 0 ? (
          <Empty description="暂无媒体" />
        ) : (
          <Image.PreviewGroup>
            <div className="grid grid-cols-3 gap-3 sm:grid-cols-4 md:grid-cols-5">
              {data?.data.map((asset) => {
                const picked = isPicked(asset);
                return (
                  <button
                    type="button"
                    key={asset.id}
                    onClick={() => toggle(asset)}
                    className={clsx(
                      'flex flex-col gap-1 rounded-md border p-2 text-left transition',
                      picked
                        ? 'border-brand ring-2 ring-brand/40'
                        : 'border-border hover:border-brand',
                    )}
                    style={{ background: 'var(--color-surface)' }}
                  >
                    {asset.mime_type.startsWith('image/') ? (
                      <Image
                        src={resolveMediaUrl(asset.storage_path)}
                        alt={asset.original_filename}
                        preview={false}
                        style={{
                          aspectRatio: '1 / 1',
                          width: '100%',
                          objectFit: 'cover',
                          borderRadius: 4,
                        }}
                      />
                    ) : (
                      <div className="grid aspect-square place-items-center rounded bg-surface-alt font-mono text-xs text-text-secondary">
                        {asset.mime_type.split('/')[1]?.toUpperCase().slice(0, 4) ?? 'FILE'}
                      </div>
                    )}
                    <div
                      className="truncate text-xs font-medium text-text"
                      title={asset.original_filename}
                    >
                      {asset.original_filename}
                    </div>
                    <div className="text-xs text-text-muted">
                      {formatBytes(asset.size)}
                    </div>
                  </button>
                );
              })}
            </div>
          </Image.PreviewGroup>
        )}
      </Spin>

      {data && data.page_count > 1 && (
        <div className="mt-4 flex justify-center">
          <Pagination
            current={page}
            pageSize={pageSize}
            total={data.total}
            showSizeChanger={false}
            onChange={setPage}
          />
        </div>
      )}

      {selected.length > 0 && (
        <div className="mt-3 flex items-center justify-between rounded border border-border bg-surface-alt px-3 py-2">
          <Space size={4} wrap>
            {selected.map((a) => (
              <span
                key={a.id}
                className="truncate font-mono text-xs text-text"
                title={a.original_filename}
              >
                {a.original_filename}
              </span>
            ))}
          </Space>
          <Button type="link" size="small" onClick={() => setSelected([])}>
            清空
          </Button>
        </div>
      )}
    </Modal>
  );
}
