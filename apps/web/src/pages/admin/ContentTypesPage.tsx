import { useState } from 'react';
import { useAsync } from '@/hooks/useAsync';
import { contentTypesApi } from '@/api';
import { LoadingSpinner } from '@/components/shared/LoadingSpinner';
import type {
  ContentTypeDefinition,
  CreateContentTypeInput,
  FieldDefinition,
  FieldType,
  UpdateContentTypeInput,
} from '@/types';

const FIELD_TYPES: FieldType[] = [
  'string', 'text', 'richtext', 'integer', 'float',
  'boolean', 'datetime', 'json', 'media', 'relation',
];

export default function ContentTypesPage() {
  const { data: types, loading, error, refetch } = useAsync(
    () => contentTypesApi.list(),
    [],
  );
  const [editing, setEditing] = useState<ContentTypeDefinition | null>(null);
  const [creating, setCreating] = useState(false);

  if (loading) return <LoadingSpinner />;
  if (error) return <div className="page-error">加载失败: {error.message}</div>;

  return (
    <div className="page">
      <div className="page-header">
        <h1>内容类型管理</h1>
        <button className="btn btn-primary" onClick={() => setCreating(true)}>
          新建内容类型
        </button>
      </div>

      {creating && (
        <ContentTypeForm
          onCancel={() => setCreating(false)}
          onSave={async (input) => {
            await contentTypesApi.create(input as CreateContentTypeInput);
            setCreating(false);
            refetch();
          }}
        />
      )}

      {editing && (
        <ContentTypeForm
          initial={editing}
          onCancel={() => setEditing(null)}
          onSave={async (input) => {
            await contentTypesApi.update(editing.api_id, input as UpdateContentTypeInput);
            setEditing(null);
            refetch();
          }}
        />
      )}

      <table className="data-table">
        <thead>
          <tr>
            <th>名称</th>
            <th>API ID</th>
            <th>类型</th>
            <th>字段数</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {types?.map((ct) => (
            <tr key={ct.id}>
              <td>{ct.name}</td>
              <td><code>{ct.api_id}</code></td>
              <td>{ct.kind}</td>
              <td>{ct.fields.length}</td>
              <td>
                <button className="btn btn-sm" onClick={() => setEditing(ct)}>
                  编辑
                </button>
                <button
                  className="btn btn-sm btn-danger"
                  onClick={async () => {
                    if (confirm(`确定删除 ${ct.name}？`)) {
                      await contentTypesApi.delete(ct.api_id);
                      refetch();
                    }
                  }}
                >
                  删除
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

// ── Field Designer ───────────────────────────────────────────────────────

function ContentTypeForm({
  initial,
  onCancel,
  onSave,
}: {
  initial?: ContentTypeDefinition;
  onCancel: () => void;
  onSave: (data: CreateContentTypeInput | UpdateContentTypeInput) => Promise<void>;
}) {
  const [name, setName] = useState(initial?.name ?? '');
  const [apiId, setApiId] = useState(initial?.api_id ?? '');
  const [description, setDescription] = useState(initial?.description ?? '');
  const [kind, setKind] = useState<'collection' | 'single'>(initial?.kind ?? 'collection');
  const [fields, setFields] = useState<FieldDefinition[]>(initial?.fields ?? []);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  function addField() {
    setFields([
      ...fields,
      {
        name: '',
        api_id: '',
        field_type: 'string',
        required: false,
        unique: false,
        localized: false,
        validation_rules: [],
      },
    ]);
  }

  function updateField(index: number, patch: Partial<FieldDefinition>) {
    setFields(fields.map((f, i) => (i === index ? { ...f, ...patch } : f)));
  }

  function removeField(index: number) {
    setFields(fields.filter((_, i) => i !== index));
  }

  async function handleSubmit() {
    setSaving(true);
    setError('');
    try {
      if (initial) {
        await onSave({ name, description: description || undefined, fields });
      } else {
        await onSave({ name, api_id: apiId, description: description || undefined, kind, fields });
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="form-overlay">
      <div className="form-card wide">
        <h2>{initial ? '编辑内容类型' : '新建内容类型'}</h2>
        {error && <div className="form-error">{error}</div>}

        <div className="form-row">
          <div className="form-group">
            <label>名称</label>
            <input value={name} onChange={(e) => setName(e.target.value)} />
          </div>
          {!initial && (
            <div className="form-group">
              <label>API ID</label>
              <input value={apiId} onChange={(e) => setApiId(e.target.value)} />
            </div>
          )}
          <div className="form-group">
            <label>描述</label>
            <input value={description} onChange={(e) => setDescription(e.target.value)} />
          </div>
          {!initial && (
            <div className="form-group">
              <label>类型</label>
              <select value={kind} onChange={(e) => setKind(e.target.value as 'collection' | 'single')}>
                <option value="collection">Collection</option>
                <option value="single">Single</option>
              </select>
            </div>
          )}
        </div>

        <h3>
          字段{' '}
          <button className="btn btn-sm" onClick={addField}>
            + 添加字段
          </button>
        </h3>

        <div className="field-list">
          {fields.map((field, i) => (
            <div key={i} className="field-row">
              <input
                placeholder="名称"
                value={field.name}
                onChange={(e) => updateField(i, { name: e.target.value })}
              />
              <input
                placeholder="API ID"
                value={field.api_id}
                onChange={(e) => updateField(i, { api_id: e.target.value })}
              />
              <select
                value={field.field_type}
                onChange={(e) =>
                  updateField(i, { field_type: e.target.value as FieldType })
                }
              >
                {FIELD_TYPES.map((ft) => (
                  <option key={ft} value={ft}>
                    {ft}
                  </option>
                ))}
              </select>
              <label>
                <input
                  type="checkbox"
                  checked={field.required}
                  onChange={(e) => updateField(i, { required: e.target.checked })}
                />
                必填
              </label>
              <label>
                <input
                  type="checkbox"
                  checked={field.unique}
                  onChange={(e) => updateField(i, { unique: e.target.checked })}
                />
                唯一
              </label>
              {field.field_type === 'relation' && (
                <input
                  placeholder="目标类型 API ID"
                  value={field.relation_target ?? ''}
                  onChange={(e) => updateField(i, { relation_target: e.target.value })}
                />
              )}
              <button className="btn btn-sm btn-danger" onClick={() => removeField(i)}>
                删除
              </button>
            </div>
          ))}
        </div>

        <div className="form-actions">
          <button className="btn" onClick={onCancel}>
            取消
          </button>
          <button className="btn btn-primary" onClick={handleSubmit} disabled={saving}>
            {saving ? '保存中...' : '保存'}
          </button>
        </div>
      </div>
    </div>
  );
}
