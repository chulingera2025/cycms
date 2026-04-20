import { useState, useMemo } from 'react';
import { useAsync } from '@/hooks/useAsync';
import { contentTypesApi, contentApi } from '@/api';
import { LoadingSpinner } from '@/components/shared/LoadingSpinner';
import type { ContentEntry, ContentTypeDefinition, ContentStatus } from '@/types';

export default function ContentPage() {
  const { data: types, loading: typesLoading } = useAsync(() => contentTypesApi.list(), []);
  const [selectedType, setSelectedType] = useState<string>('');
  const [page, setPage] = useState(1);
  const [statusFilter, setStatusFilter] = useState<ContentStatus | ''>('');
  const [search, setSearch] = useState('');
  const [editingEntry, setEditingEntry] = useState<ContentEntry | null>(null);
  const [creating, setCreating] = useState(false);

  const typeApiId = selectedType || types?.[0]?.api_id || '';
  const params = useMemo(() => {
    const p: Record<string, string> = { page: String(page), pageSize: '20' };
    if (statusFilter) p['status'] = statusFilter;
    if (search) p['filter[slug][contains]'] = search;
    return p;
  }, [page, statusFilter, search]);

  const {
    data: entries,
    loading: entriesLoading,
    refetch,
  } = useAsync(
    () => (typeApiId ? contentApi.list(typeApiId, params) : Promise.resolve(null)),
    [typeApiId, params],
  );

  const currentType = types?.find((t) => t.api_id === typeApiId);

  if (typesLoading) return <LoadingSpinner />;

  return (
    <div className="page">
      <div className="page-header">
        <h1>内容管理</h1>
        <div className="header-actions">
          <select value={typeApiId} onChange={(e) => { setSelectedType(e.target.value); setPage(1); }}>
            {types?.map((t) => (
              <option key={t.api_id} value={t.api_id}>{t.name}</option>
            ))}
          </select>
          <select value={statusFilter} onChange={(e) => { setStatusFilter(e.target.value as ContentStatus | ''); setPage(1); }}>
            <option value="">全部状态</option>
            <option value="draft">草稿</option>
            <option value="published">已发布</option>
            <option value="archived">已归档</option>
          </select>
          <input placeholder="搜索 slug..." value={search} onChange={(e) => setSearch(e.target.value)} />
          <button className="btn btn-primary" onClick={() => setCreating(true)}>
            新建
          </button>
        </div>
      </div>

      {creating && currentType && (
        <EntryForm
          contentType={currentType}
          onCancel={() => setCreating(false)}
          onSave={async (data) => {
            await contentApi.create(typeApiId, data);
            setCreating(false);
            refetch();
          }}
        />
      )}

      {editingEntry && currentType && (
        <EntryForm
          contentType={currentType}
          initial={editingEntry}
          onCancel={() => setEditingEntry(null)}
          onSave={async (data) => {
            await contentApi.update(typeApiId, editingEntry.id, data);
            setEditingEntry(null);
            refetch();
          }}
        />
      )}

      {entriesLoading ? (
        <LoadingSpinner />
      ) : (
        <>
          <table className="data-table">
            <thead>
              <tr>
                <th>ID</th>
                <th>Slug</th>
                <th>状态</th>
                <th>创建时间</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              {entries?.data.map((entry) => (
                <tr key={entry.id}>
                  <td title={entry.id}>{entry.id.slice(0, 8)}...</td>
                  <td>{entry.slug ?? '—'}</td>
                  <td>
                    <span className={`status-badge status-${entry.status}`}>
                      {entry.status}
                    </span>
                  </td>
                  <td>{new Date(entry.created_at).toLocaleString()}</td>
                  <td className="action-cell">
                    <button className="btn btn-sm" onClick={() => setEditingEntry(entry)}>
                      编辑
                    </button>
                    {entry.status === 'draft' && (
                      <button
                        className="btn btn-sm btn-success"
                        onClick={async () => {
                          await contentApi.publish(typeApiId, entry.id);
                          refetch();
                        }}
                      >
                        发布
                      </button>
                    )}
                    {entry.status === 'published' && (
                      <button
                        className="btn btn-sm btn-warning"
                        onClick={async () => {
                          await contentApi.unpublish(typeApiId, entry.id);
                          refetch();
                        }}
                      >
                        撤回
                      </button>
                    )}
                    <button
                      className="btn btn-sm btn-danger"
                      onClick={async () => {
                        if (confirm('确定删除？')) {
                          await contentApi.delete(typeApiId, entry.id);
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

          {entries && entries.meta.page_count > 1 && (
            <div className="pagination">
              <button disabled={page <= 1} onClick={() => setPage(page - 1)}>
                上一页
              </button>
              <span>
                {entries.meta.page} / {entries.meta.page_count}（共 {entries.meta.total} 条）
              </span>
              <button
                disabled={page >= entries.meta.page_count}
                onClick={() => setPage(page + 1)}
              >
                下一页
              </button>
            </div>
          )}
        </>
      )}
    </div>
  );
}

// ── Entry Form ───────────────────────────────────────────────────────────

function EntryForm({
  contentType,
  initial,
  onCancel,
  onSave,
}: {
  contentType: ContentTypeDefinition;
  initial?: ContentEntry;
  onCancel: () => void;
  onSave: (data: { data: Record<string, unknown>; slug?: string }) => Promise<void>;
}) {
  const [slug, setSlug] = useState(initial?.slug ?? '');
  const [fields, setFields] = useState<Record<string, unknown>>(
    initial ? (initial.fields as Record<string, unknown>) : {},
  );
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  function updateField(apiId: string, value: unknown) {
    setFields({ ...fields, [apiId]: value });
  }

  async function handleSubmit() {
    setSaving(true);
    setError('');
    try {
      await onSave({ data: fields, slug: slug || undefined });
    } catch (err) {
      setError(err instanceof Error ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="form-overlay">
      <div className="form-card wide">
        <h2>{initial ? '编辑内容' : '新建内容'}</h2>
        {error && <div className="form-error">{error}</div>}

        <div className="form-group">
          <label>Slug</label>
          <input value={slug} onChange={(e) => setSlug(e.target.value)} placeholder="可选" />
        </div>

        {contentType.fields.map((fd) => (
          <div key={fd.api_id} className="form-group">
            <label>
              {fd.name}
              {fd.required && <span className="required">*</span>}
              <small> ({fd.field_type})</small>
            </label>
            <FieldInput
              fieldDef={fd}
              value={fields[fd.api_id]}
              onChange={(v) => updateField(fd.api_id, v)}
            />
          </div>
        ))}

        <div className="form-actions">
          <button className="btn" onClick={onCancel}>取消</button>
          <button className="btn btn-primary" onClick={handleSubmit} disabled={saving}>
            {saving ? '保存中...' : '保存'}
          </button>
        </div>
      </div>
    </div>
  );
}

function FieldInput({
  fieldDef,
  value,
  onChange,
}: {
  fieldDef: { field_type: string };
  value: unknown;
  onChange: (v: unknown) => void;
}) {
  switch (fieldDef.field_type) {
    case 'boolean':
      return (
        <input
          type="checkbox"
          checked={!!value}
          onChange={(e) => onChange(e.target.checked)}
        />
      );
    case 'integer':
    case 'float':
      return (
        <input
          type="number"
          value={value != null ? String(value) : ''}
          onChange={(e) =>
            onChange(
              fieldDef.field_type === 'integer'
                ? parseInt(e.target.value, 10) || 0
                : parseFloat(e.target.value) || 0,
            )
          }
        />
      );
    case 'text':
    case 'richtext':
      return (
        <textarea
          value={typeof value === 'string' ? value : ''}
          onChange={(e) => onChange(e.target.value)}
          rows={5}
        />
      );
    case 'datetime':
      return (
        <input
          type="datetime-local"
          value={typeof value === 'string' ? value.slice(0, 16) : ''}
          onChange={(e) => onChange(e.target.value ? new Date(e.target.value).toISOString() : null)}
        />
      );
    case 'json':
      return (
        <textarea
          value={typeof value === 'string' ? value : JSON.stringify(value ?? '', null, 2)}
          onChange={(e) => {
            try { onChange(JSON.parse(e.target.value)); } catch { /* keep raw */ }
          }}
          rows={4}
        />
      );
    default:
      return (
        <input
          type="text"
          value={typeof value === 'string' ? value : (value != null ? String(value) : '')}
          onChange={(e) => onChange(e.target.value)}
        />
      );
  }
}
