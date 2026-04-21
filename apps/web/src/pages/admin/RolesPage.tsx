import { useState } from 'react';
import { useAsync } from '@/hooks/useAsync';
import { rolesApi } from '@/lib/api';
import { LoadingSpinner } from '@/components/shared/LoadingSpinner';
import type { Role, Permission, CreateRoleInput } from '@/types';

export default function RolesPage() {
  const { data: roles, loading, error, refetch } = useAsync(() => rolesApi.list(), []);
  const { data: allPermissions } = useAsync(() => rolesApi.listPermissions(), []);
  const [editing, setEditing] = useState<Role | null>(null);
  const [creating, setCreating] = useState(false);

  if (loading) return <LoadingSpinner />;
  if (error) return <div className="page-error">加载失败: {error.message}</div>;

  return (
    <div className="page">
      <div className="page-header">
        <h1>角色与权限</h1>
        <button className="btn btn-primary" onClick={() => setCreating(true)}>
          新建角色
        </button>
      </div>

      {creating && (
        <RoleForm
          allPermissions={allPermissions ?? []}
          onCancel={() => setCreating(false)}
          onSave={async (data) => {
            await rolesApi.create(data);
            setCreating(false);
            refetch();
          }}
        />
      )}

      {editing && (
        <RoleForm
          initial={editing}
          allPermissions={allPermissions ?? []}
          onCancel={() => setEditing(null)}
          onSave={async (data) => {
            await rolesApi.update(editing.id, data);
            setEditing(null);
            refetch();
          }}
        />
      )}

      <div className="roles-grid">
        {roles?.map((role) => (
          <div key={role.id} className="role-card">
            <div className="role-header">
              <h3>{role.name}</h3>
              {role.is_system && <span className="badge">系统</span>}
            </div>
            {role.description && <p>{role.description}</p>}
            <div className="permission-list">
              {role.permissions.map((p) => (
                <span key={p.id} className="permission-tag">{p.code}</span>
              ))}
            </div>
            <div className="role-actions">
              <button className="btn btn-sm" onClick={() => setEditing(role)}>编辑</button>
              {!role.is_system && (
                <button
                  className="btn btn-sm btn-danger"
                  onClick={async () => {
                    if (confirm(`确定删除 ${role.name}？`)) {
                      await rolesApi.delete(role.id);
                      refetch();
                    }
                  }}
                >
                  删除
                </button>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function RoleForm({
  initial,
  allPermissions,
  onCancel,
  onSave,
}: {
  initial?: Role;
  allPermissions: Permission[];
  onCancel: () => void;
  onSave: (data: CreateRoleInput) => Promise<void>;
}) {
  const [name, setName] = useState(initial?.name ?? '');
  const [description, setDescription] = useState(initial?.description ?? '');
  const [selectedPerms, setSelectedPerms] = useState<string[]>(
    initial?.permissions.map((p) => p.id) ?? [],
  );
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  // 按 domain 分组
  const grouped = allPermissions.reduce<Record<string, Permission[]>>((acc, p) => {
    const domain = p.code.split('.')[0] ?? 'other';
    (acc[domain] ??= []).push(p);
    return acc;
  }, {});

  function togglePerm(id: string) {
    setSelectedPerms((prev) =>
      prev.includes(id) ? prev.filter((p) => p !== id) : [...prev, id],
    );
  }

  async function handleSubmit() {
    setSaving(true);
    setError('');
    try {
      await onSave({ name, description: description || undefined, permission_ids: selectedPerms });
    } catch (err) {
      setError(err instanceof Error ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="form-overlay">
      <div className="form-card wide">
        <h2>{initial ? '编辑角色' : '新建角色'}</h2>
        {error && <div className="form-error">{error}</div>}

        <div className="form-group">
          <label>名称</label>
          <input value={name} onChange={(e) => setName(e.target.value)} required />
        </div>
        <div className="form-group">
          <label>描述</label>
          <input value={description} onChange={(e) => setDescription(e.target.value)} />
        </div>

        <h3>权限矩阵</h3>
        <div className="permission-matrix">
          {Object.entries(grouped).map(([domain, perms]) => (
            <div key={domain} className="perm-domain">
              <h4>{domain}</h4>
              <div className="perm-checks">
                {perms.map((p) => (
                  <label key={p.id}>
                    <input
                      type="checkbox"
                      checked={selectedPerms.includes(p.id)}
                      onChange={() => togglePerm(p.id)}
                    />
                    {p.code.split('.').slice(1).join('.')}
                  </label>
                ))}
              </div>
            </div>
          ))}
        </div>

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
