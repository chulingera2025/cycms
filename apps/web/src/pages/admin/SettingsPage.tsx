import { useState } from 'react';
import { useAsync } from '@/hooks/useAsync';
import { settingsApi } from '@/api';
import { LoadingSpinner } from '@/components/shared/LoadingSpinner';
import type { SettingsEntry } from '@/types';

const NAMESPACES = ['system', 'content', 'media', 'auth'];

export default function SettingsPage() {
  const [namespace, setNamespace] = useState('system');
  const { data: settings, loading, refetch } = useAsync(
    () => settingsApi.get(namespace),
    [namespace],
  );
  const [editKey, setEditKey] = useState<string | null>(null);
  const [editValue, setEditValue] = useState('');
  const [newKey, setNewKey] = useState('');
  const [newValue, setNewValue] = useState('');

  function startEdit(entry: SettingsEntry) {
    setEditKey(entry.key);
    setEditValue(typeof entry.value === 'string' ? entry.value : JSON.stringify(entry.value, null, 2));
  }

  async function saveEdit() {
    if (!editKey) return;
    try {
      const value = JSON.parse(editValue);
      await settingsApi.set(namespace, editKey, value);
    } catch {
      await settingsApi.set(namespace, editKey, editValue);
    }
    setEditKey(null);
    refetch();
  }

  async function addNew() {
    if (!newKey) return;
    let value: unknown = newValue;
    try { value = JSON.parse(newValue); } catch { /* use as string */ }
    await settingsApi.set(namespace, newKey, value);
    setNewKey('');
    setNewValue('');
    refetch();
  }

  return (
    <div className="page">
      <div className="page-header">
        <h1>系统设置</h1>
        <select value={namespace} onChange={(e) => setNamespace(e.target.value)}>
          {NAMESPACES.map((ns) => (
            <option key={ns} value={ns}>{ns}</option>
          ))}
        </select>
      </div>

      {loading ? (
        <LoadingSpinner />
      ) : (
        <table className="data-table">
          <thead>
            <tr>
              <th>键</th>
              <th>值</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            {settings?.map((entry) => (
              <tr key={entry.key}>
                <td><code>{entry.key}</code></td>
                <td>
                  {editKey === entry.key ? (
                    <textarea
                      value={editValue}
                      onChange={(e) => setEditValue(e.target.value)}
                      rows={3}
                    />
                  ) : (
                    <pre className="settings-value">
                      {typeof entry.value === 'string'
                        ? entry.value
                        : JSON.stringify(entry.value, null, 2)}
                    </pre>
                  )}
                </td>
                <td className="action-cell">
                  {editKey === entry.key ? (
                    <>
                      <button className="btn btn-sm btn-primary" onClick={saveEdit}>保存</button>
                      <button className="btn btn-sm" onClick={() => setEditKey(null)}>取消</button>
                    </>
                  ) : (
                    <>
                      <button className="btn btn-sm" onClick={() => startEdit(entry)}>编辑</button>
                      <button
                        className="btn btn-sm btn-danger"
                        onClick={async () => {
                          if (confirm(`确定删除 ${entry.key}？`)) {
                            await settingsApi.delete(namespace, entry.key);
                            refetch();
                          }
                        }}
                      >
                        删除
                      </button>
                    </>
                  )}
                </td>
              </tr>
            ))}
            <tr>
              <td>
                <input placeholder="新键" value={newKey} onChange={(e) => setNewKey(e.target.value)} />
              </td>
              <td>
                <input placeholder="新值" value={newValue} onChange={(e) => setNewValue(e.target.value)} />
              </td>
              <td>
                <button className="btn btn-sm btn-primary" onClick={addNew} disabled={!newKey}>
                  添加
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      )}
    </div>
  );
}
