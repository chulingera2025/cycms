import { useEffect, useState } from 'react';
import { Alert, Button, Drawer, Form, Input, Space } from 'antd';
import { FieldRenderer } from './FieldRenderer';
import { ApiError } from '@/lib/api/client';
import type { ContentEntry, ContentTypeDefinition } from '@/types';

interface Props {
  open: boolean;
  contentType: ContentTypeDefinition | null;
  initial?: ContentEntry | null;
  onClose: () => void;
  onSubmit: (payload: {
    data: Record<string, unknown>;
    slug?: string | null;
  }) => Promise<void>;
  loading?: boolean;
}

export function EntryEditor({
  open,
  contentType,
  initial,
  onClose,
  onSubmit,
  loading,
}: Props) {
  const isEdit = Boolean(initial);
  const [slug, setSlug] = useState('');
  const [fields, setFields] = useState<Record<string, unknown>>({});
  const [submitError, setSubmitError] = useState('');

  useEffect(() => {
    if (open) {
      setSlug(initial?.slug ?? '');
      setFields((initial?.fields as Record<string, unknown>) ?? {});
      setSubmitError('');
    }
  }, [open, initial]);

  if (!contentType) return null;

  function handleChange(apiId: string, value: unknown) {
    setFields((prev) => ({ ...prev, [apiId]: value }));
  }

  async function handleSubmit() {
    setSubmitError('');
    if (!contentType) return;
    const missing = contentType.fields.filter(
      (f) => f.required && (fields[f.api_id] == null || fields[f.api_id] === ''),
    );
    if (missing.length > 0) {
      setSubmitError(`以下字段必填：${missing.map((m) => m.name).join('、')}`);
      return;
    }
    const parsed: Record<string, unknown> = {};
    for (const f of contentType.fields) {
      const v = fields[f.api_id];
      if (f.field_type === 'json' && typeof v === 'string' && v !== '') {
        try {
          parsed[f.api_id] = JSON.parse(v);
        } catch {
          setSubmitError(`字段 ${f.name} 不是合法 JSON`);
          return;
        }
      } else {
        parsed[f.api_id] = v;
      }
    }
    try {
      await onSubmit({ data: parsed, slug: slug || null });
    } catch (err) {
      setSubmitError(err instanceof ApiError ? err.message : '保存失败');
    }
  }

  return (
    <Drawer
      open={open}
      title={isEdit ? `编辑 ${contentType.name}` : `新建 ${contentType.name}`}
      width={960}
      onClose={onClose}
      destroyOnClose
      extra={
        <Space>
          <Button onClick={onClose}>取消</Button>
          <Button type="primary" loading={loading} onClick={handleSubmit}>
            保存
          </Button>
        </Space>
      }
    >
      {submitError && (
        <Alert
          type="error"
          message={submitError}
          showIcon
          style={{ marginBottom: 16 }}
        />
      )}
      <Form layout="vertical">
        <Form.Item label="Slug" extra="可选，URL 友好标识；留空由后端生成">
          <Input
            value={slug}
            onChange={(e) => setSlug(e.target.value)}
            placeholder="my-first-post"
          />
        </Form.Item>
        {contentType.fields.map((fd) => (
          <Form.Item
            key={fd.api_id}
            label={
              <span>
                {fd.name}
                {fd.required && <span className="ml-1 text-danger">*</span>}
                <span className="ml-2 font-mono text-xs text-text-muted">
                  {fd.field_type}
                </span>
              </span>
            }
            extra={fd.description}
          >
            <FieldRenderer
              field={fd}
              value={fields[fd.api_id]}
              onChange={(v) => handleChange(fd.api_id, v)}
            />
          </Form.Item>
        ))}
      </Form>
    </Drawer>
  );
}
