import { DatePicker, Input, InputNumber, Switch } from 'antd';
import dayjs from 'dayjs';
import MDEditor from '@uiw/react-md-editor';
import type { FieldDefinition } from '@/types';

interface Props {
  field: FieldDefinition;
  value: unknown;
  onChange: (v: unknown) => void;
}

// TODO!!! media / relation 字段目前仅接受 ID 原文；后续补 MediaPicker 与远程搜索 Select
export function FieldRenderer({ field, value, onChange }: Props) {
  switch (field.field_type) {
    case 'boolean':
      return <Switch checked={Boolean(value)} onChange={onChange} />;

    case 'integer':
      return (
        <InputNumber
          style={{ width: '100%' }}
          precision={0}
          value={typeof value === 'number' ? value : undefined}
          onChange={(v) => onChange(v ?? null)}
        />
      );

    case 'float':
      return (
        <InputNumber
          style={{ width: '100%' }}
          value={typeof value === 'number' ? value : undefined}
          onChange={(v) => onChange(v ?? null)}
        />
      );

    case 'text':
      return (
        <Input.TextArea
          value={typeof value === 'string' ? value : ''}
          onChange={(e) => onChange(e.target.value)}
          autoSize={{ minRows: 3, maxRows: 10 }}
        />
      );

    case 'richtext':
      return (
        <div data-color-mode="inherit">
          <MDEditor
            value={typeof value === 'string' ? value : ''}
            onChange={(v) => onChange(v ?? '')}
            height={280}
            preview="live"
          />
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
      return (
        <Input
          placeholder="媒体资源 ID"
          value={typeof value === 'string' ? value : ''}
          onChange={(e) => onChange(e.target.value)}
        />
      );

    case 'relation':
      return (
        <Input
          placeholder={`关联 ${field.relation_target ?? ''} 的 ID`}
          value={typeof value === 'string' ? value : ''}
          onChange={(e) => onChange(e.target.value)}
        />
      );

    default:
      return (
        <Input
          value={typeof value === 'string' ? value : ''}
          onChange={(e) => onChange(e.target.value)}
        />
      );
  }
}
