import { useState } from 'react';
import { useAsync } from '@/hooks/useAsync';
import { usersApi, rolesApi } from '@/lib/api';
import { LoadingSpinner } from '@/components/shared/LoadingSpinner';
import type { User, CreateUserInput, Role } from '@/types';

export default function UsersPage() {
  const { data: users, loading, error, refetch } = useAsync(() => usersApi.list(), []);
  const { data: roles } = useAsync(() => rolesApi.list(), []);
  const [creating, setCreating] = useState(false);
  const [editing, setEditing] = useState<User | null>(null);

  if (loading) return <LoadingSpinner />;
  if (error) return <div className="page-error">加载失败: {error.message}</div>;

  return (
    <div className="page">
      <div className="page-header">
        <h1>用户管理</h1>
        <button className="btn btn-primary" onClick={() => setCreating(true)}>
          新建用户
        </button>
      </div>

      {creating && (
        <UserForm
          roles={roles ?? []}
          onCancel={() => setCreating(false)}
          onSave={async (data) => {
            await usersApi.create(data);
            setCreating(false);
            refetch();
          }}
        />
      )}

      {editing && (
        <UserForm
          initial={editing}
          roles={roles ?? []}
          onCancel={() => setEditing(null)}
          onSave={async (data) => {
            await usersApi.update(editing.id, data);
            setEditing(null);
            refetch();
          }}
        />
      )}

      <table className="data-table">
        <thead>
          <tr>
            <th>用户名</th>
            <th>邮箱</th>
            <th>角色</th>
            <th>状态</th>
            <th>创建时间</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {users?.map((u) => (
            <tr key={u.id}>
              <td>{u.username}</td>
              <td>{u.email}</td>
              <td>{u.roles.join(', ')}</td>
              <td>{u.is_active ? '活跃' : '已禁用'}</td>
              <td>{new Date(u.created_at).toLocaleString()}</td>
              <td className="action-cell">
                <button className="btn btn-sm" onClick={() => setEditing(u)}>编辑</button>
                <button
                  className="btn btn-sm btn-danger"
                  onClick={async () => {
                    if (confirm(`确定删除 ${u.username}？`)) {
                      await usersApi.delete(u.id);
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

function UserForm({
  initial,
  roles,
  onCancel,
  onSave,
}: {
  initial?: User;
  roles: Role[];
  onCancel: () => void;
  onSave: (data: CreateUserInput) => Promise<void>;
}) {
  const [username, setUsername] = useState(initial?.username ?? '');
  const [email, setEmail] = useState(initial?.email ?? '');
  const [password, setPassword] = useState('');
  const [isActive, setIsActive] = useState(initial?.is_active ?? true);
  const [selectedRoles, setSelectedRoles] = useState<string[]>(initial?.role_ids ?? []);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  function toggleRole(roleId: string) {
    setSelectedRoles((prev) =>
      prev.includes(roleId) ? prev.filter((r) => r !== roleId) : [...prev, roleId],
    );
  }

  async function handleSubmit() {
    setSaving(true);
    setError('');
    try {
      const data: CreateUserInput = {
        username,
        email,
        password: password || (initial ? undefined! : password),
        is_active: isActive,
        role_ids: selectedRoles,
      };
      if (initial && !password) {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        delete (data as any).password;
      }
      await onSave(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="form-overlay">
      <div className="form-card">
        <h2>{initial ? '编辑用户' : '新建用户'}</h2>
        {error && <div className="form-error">{error}</div>}

        <div className="form-group">
          <label>用户名</label>
          <input value={username} onChange={(e) => setUsername(e.target.value)} required />
        </div>
        <div className="form-group">
          <label>邮箱</label>
          <input type="email" value={email} onChange={(e) => setEmail(e.target.value)} required />
        </div>
        <div className="form-group">
          <label>{initial ? '新密码（留空不修改）' : '密码'}</label>
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required={!initial}
            autoComplete="new-password"
          />
        </div>
        <div className="form-group">
          <label>
            <input
              type="checkbox"
              checked={isActive}
              onChange={(e) => setIsActive(e.target.checked)}
            />
            活跃
          </label>
        </div>
        <div className="form-group">
          <label>角色</label>
          <div className="role-checkboxes">
            {roles.map((role) => (
              <label key={role.id}>
                <input
                  type="checkbox"
                  checked={selectedRoles.includes(role.id)}
                  onChange={() => toggleRole(role.id)}
                />
                {role.name}
              </label>
            ))}
          </div>
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
