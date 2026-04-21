import { lazy, Suspense, useState } from 'react';
import { Button, DatePicker, Input, InputNumber, Skeleton, Space, Switch } from 'antd';
import dayjs from 'dayjs';
import { Image as ImageIcon, Link2, X } from 'lucide-react';
import { MediaPicker } from '@/features/media/MediaPicker';
import {
  getFieldTypeKind,
  getRelationConfig,
  isMultiRelationField,
} from '@/features/content-types/fieldType';
import { useMedia } from '@/features/media/hooks';
import { resolveMediaUrl, formatBytes } from '@/utils/format';
import type { FieldDefinition } from '@/types';
import { RelationSelect } from './RelationSelect';

const MDEditor = lazy(() => import('@uiw/react-md-editor'));

interface Props {
  field: FieldDefinition;
  value: unknown;
  onChange: (v: unknown) => void;
}

function MediaField({ value, onChange }: { value: unknown; onChange: (v: unknown) => void }) {
  const [open, setOpen] = useState(false);
  const id = typeof value === 'string' && value ? value : null;
  const { data: asset, isLoading } = useMedia(id);

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <Button icon={<ImageIcon size={14} />} onClick={() => setOpen(true)}>
          {id ? '更换媒体' : '选择媒体'}
        </Button>
        {id && (
          <Button type="text" icon={<X size={14} />} onClick={() => onChange(null)}>
            清除
          </Button>
        )}
      </div>
      {id &&
        (isLoading ? (
          <Skeleton.Input active size="small" style={{ width: 240 }} />
        ) : asset ? (
          <div className="flex items-center gap-3 rounded border border-border bg-surface-alt p-2">
            {asset.mime_type.startsWith('image/') ? (
              <img
                src={resolveMediaUrl(asset.storage_path)}
                alt={asset.original_filename}
                className="h-12 w-12 rounded object-cover"
              />
            ) : (
              <div className="grid h-12 w-12 place-items-center rounded bg-surface font-mono text-xs text-text-secondary">
                {asset.mime_type.split('/')[1]?.toUpperCase().slice(0, 4) ?? 'FILE'}
              </div>
            )}
            <div className="min-w-0">
              <div
                className="truncate text-sm font-medium text-text"
                title={asset.original_filename}
              >
                {asset.original_filename}
              </div>
              <div className="font-mono text-xs text-text-muted">
                {formatBytes(asset.size)} · {asset.mime_type}
              </div>
            </div>
          </div>
        ) : (
          <div className="font-mono text-xs text-text-muted">{id}</div>
        ))}
      <MediaPicker
        open={open}
        onClose={() => setOpen(false)}
        onSelect={(ids) => onChange(ids[0] ?? null)}
        initialSelected={id ? [id] : []}
      />
    </div>
  );
}

export function FieldRenderer({ field, value, onChange }: Props) {
  const fieldTypeKind = getFieldTypeKind(field.field_type);

  switch (fieldTypeKind) {
    case 'boolean':
      return <Switch checked={Boolean(value)} onChange={onChange} />;

    case 'number':
      return (
        <InputNumber
          style={{ width: '100%' }}
          precision={field.field_type.kind === 'number' && !field.field_type.decimal ? 0 : undefined}
          value={typeof value === 'number' ? value : undefined}
          onChange={(v) => onChange(v ?? null)}
        />
      );

    case 'text':
      return (
        <Input
          value={typeof value === 'string' ? value : ''}
          onChange={(e) => onChange(e.target.value)}
        />
      );

    case 'richtext':
      return (
        <div data-color-mode="inherit">
          <Suspense fallback={<Skeleton.Input active block style={{ height: 280 }} />}>
            <MDEditor
              value={typeof value === 'string' ? value : ''}
              onChange={(v) => onChange(v ?? '')}
              height={280}
              preview="live"
            />
          </Suspense>
        </div>
      );

    case 'datetime':
      return (
        <DatePicker
          style={{ width: '100%' }}
          showTime
          value={typeof value === 'string' && value ? dayjs(value) : null}
          onChange={(d) => onChange(d ? d.toISOString() : null)}
        />
      );

    case 'json':
      return (
        <Input.TextArea
          style={{ fontFamily: 'var(--font-mono)', fontSize: 12 }}
          value={
            typeof value === 'string' ? value : JSON.stringify(value ?? '', null, 2)
          }
          onChange={(e) => onChange(e.target.value)}
          autoSize={{ minRows: 4, maxRows: 16 }}
        />
      );

    case 'media':
      return <MediaField value={value} onChange={onChange} />;

    case 'relation': {
      const multiple = isMultiRelationField(field.field_type);
      const relation = getRelationConfig(field.field_type);
      const normalized = multiple
        ? Array.isArray(value)
          ? (value as string[])
          : value
            ? [String(value)]
            : []
        : typeof value === 'string'
          ? value
          : null;
      if (!relation.targetType) {
        return (
          <Space>
            <Link2 size={14} className="text-text-muted" />
            <span className="text-xs text-text-muted">未配置 relation_target</span>
          </Space>
        );
      }
      return (
        <RelationSelect
          target={relation.targetType}
          multiple={multiple}
          value={normalized}
          onChange={onChange}
        />
      );
    }

    default:
      return (
        <Input
          value={typeof value === 'string' ? value : ''}
          onChange={(e) => onChange(e.target.value)}
        />
      );
  }
}
