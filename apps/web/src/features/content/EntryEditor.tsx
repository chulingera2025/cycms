import { useMemo, useEffect, useState } from 'react';
import { Alert, Button, Drawer, Form, Input, Space } from 'antd';
import { useAdminExtensions } from '@/features/admin-extensions';
import { PluginSlotHost } from '@/features/admin-extensions/module-host';
import { FieldRenderer } from './FieldRenderer';
import {
  buildHostFieldError,
  isEntryFieldDirty,
  resolveInitialFields,
} from './editorState';
import { getFieldTypeKind, getFieldTypeLabel } from '@/features/content-types/fieldType';
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

function parseLooseCustomValue(value: string): unknown {
  const trimmed = value.trim();
  if (!trimmed) {
    return value;
  }

  try {
    return JSON.parse(trimmed);
  } catch {
    return value;
  }
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
  const { bootstrap, getSlots } = useAdminExtensions();
  const [slug, setSlug] = useState('');
  const [fields, setFields] = useState<Record<string, unknown>>({});
  const [touchedFields, setTouchedFields] = useState<string[]>([]);
  const [submitAttempted, setSubmitAttempted] = useState(false);
  const [pluginFieldErrors, setPluginFieldErrors] = useState<Record<string, string>>({});
  const [submitError, setSubmitError] = useState('');
  const contentTypeApiId = contentType?.api_id ?? '';
  const initialFields = useMemo(() => resolveInitialFields(initial), [initial]);

  const sidebarSlots = useMemo(
    () => (contentTypeApiId ? getSlots('content.editor.sidebar', contentTypeApiId) : []),
    [contentTypeApiId, getSlots],
  );

  useEffect(() => {
    if (open) {
      setSlug(initial?.slug ?? '');
      setFields(initialFields);
      setTouchedFields([]);
      setSubmitAttempted(false);
      setPluginFieldErrors({});
      setSubmitError('');
    }
  }, [initial, initialFields, open]);

  if (!contentType) return null;

  const hasSidebarSlots = sidebarSlots.length > 0;
  const touchedFieldSet = new Set(touchedFields);
  const dirtyFields = contentType.fields
    .filter((field) => isEntryFieldDirty(initialFields, field.api_id, fields[field.api_id]))
    .map((field) => field.api_id);
  const hostFieldErrors = Object.fromEntries(
    contentType.fields.map((field) => [field.api_id, buildHostFieldError(field, fields[field.api_id])]),
  ) as Record<string, string | null>;
  const validationErrors = Object.fromEntries(
    contentType.fields.map((field) => {
      const hostError = touchedFieldSet.has(field.api_id) || submitAttempted ? hostFieldErrors[field.api_id] : null;
      return [field.api_id, pluginFieldErrors[field.api_id] ?? hostError ?? null];
    }),
  ) as Record<string, string | null>;
  const isDirty = dirtyFields.length > 0 || slug !== (initial?.slug ?? '');

  function handleChange(apiId: string, value: unknown) {
    setTouchedFields((prev) => (prev.includes(apiId) ? prev : [...prev, apiId]));
    setFields((prev) => ({ ...prev, [apiId]: value }));
  }

  function setFieldError(apiId: string, message: string | null) {
    setPluginFieldErrors((prev) => {
      const next = { ...prev };
      if (message) {
        next[apiId] = message;
      } else {
        delete next[apiId];
      }
      return next;
    });
  }

  function validateField(apiId: string) {
    setTouchedFields((prev) => (prev.includes(apiId) ? prev : [...prev, apiId]));
    return pluginFieldErrors[apiId] ?? hostFieldErrors[apiId] ?? null;
  }

  async function handleSubmit() {
    setSubmitError('');
    setSubmitAttempted(true);
    if (!contentType) return;
    setTouchedFields(contentType.fields.map((field) => field.api_id));
    const blockingErrors = contentType.fields
      .map((field) => pluginFieldErrors[field.api_id] ?? hostFieldErrors[field.api_id] ?? null)
      .filter((message): message is string => Boolean(message));
    const firstBlockingError = blockingErrors[0];
    if (firstBlockingError) {
      setSubmitError(firstBlockingError);
      return;
    }
    const parsed: Record<string, unknown> = {};
    for (const f of contentType.fields) {
      const v = fields[f.api_id];
      if (getFieldTypeKind(f.field_type) === 'json' && typeof v === 'string' && v !== '') {
        try {
          parsed[f.api_id] = JSON.parse(v);
        } catch {
          setSubmitError(`字段 ${f.name} 不是合法 JSON`);
          return;
        }
      } else if (getFieldTypeKind(f.field_type) === 'custom' && typeof v === 'string') {
        parsed[f.api_id] = parseLooseCustomValue(v);
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
      <div
        className={
          hasSidebarSlots
            ? 'grid grid-cols-1 gap-6 lg:grid-cols-[minmax(0,1fr)_320px]'
            : 'grid grid-cols-1 gap-6'
        }
      >
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
                    {getFieldTypeLabel(fd.field_type)}
                  </span>
                </span>
              }
              extra={fd.description}
              help={validationErrors[fd.api_id] ?? undefined}
              validateStatus={validationErrors[fd.api_id] ? 'error' : undefined}
            >
              <FieldRenderer
                field={fd}
                value={fields[fd.api_id]}
                onChange={(v) => handleChange(fd.api_id, v)}
                contentTypeApiId={contentType.api_id}
                entryId={initial?.id}
                mode={isEdit ? 'edit' : 'create'}
                dirty={dirtyFields.includes(fd.api_id)}
                validationError={validationErrors[fd.api_id]}
                setValidationError={(message) => setFieldError(fd.api_id, message)}
                validate={() => validateField(fd.api_id)}
              />
            </Form.Item>
          ))}
        </Form>

        {hasSidebarSlots && (
          <aside className="space-y-3">
            <div>
              <h3 className="m-0 text-sm font-semibold text-text">插件侧边栏</h3>
              <p className="mt-1 text-xs text-text-muted">
                当前区域由已启用插件通过 `content.editor.sidebar` 扩展点注入。
              </p>
              <p className="mt-1 text-xs text-text-muted">
                当前表单{isDirty ? '存在未保存改动' : '尚未产生未保存改动'}，宿主会同步暴露 dirty-state 与校验错误给插件模块。
              </p>
            </div>
            {sidebarSlots.map((slot) => (
              <PluginSlotHost
                key={`${slot.pluginName}:${slot.contribution.id}`}
                pluginName={slot.pluginName}
                contributionId={slot.contribution.id}
                slotId={slot.contribution.slot}
                sdkVersion={bootstrap?.shellSdkVersion ?? '1.0.0'}
                moduleUrl={slot.contribution.moduleUrl}
                styles={slot.contribution.styles}
                contentTypeApiId={contentType.api_id}
                values={fields}
                dirtyFields={dirtyFields}
                validationErrors={validationErrors}
                setFieldValue={handleChange}
                setFieldError={setFieldError}
                getFieldError={(apiId) => validationErrors[apiId] ?? null}
                validateField={validateField}
                entryId={initial?.id}
                mode={isEdit ? 'edit' : 'create'}
              />
            ))}
          </aside>
        )}
      </div>
    </Drawer>
  );
}
