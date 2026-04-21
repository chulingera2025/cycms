import { useState, type FormEvent } from 'react';
import { useAuth } from '@/stores/auth';
import { LoadingSpinner } from '@/components/shared/LoadingSpinner';
import { ApiError } from '@/lib/api/client';
import { usersApi } from '@/lib/api';

export default function ProfilePage() {
  const { user, refresh, logout } = useAuth();
  const [editing, setEditing] = useState(false);
  const [email, setEmail] = useState(user?.email ?? '');
  const [error, setError] = useState('');
  const [saving, setSaving] = useState(false);

  if (!user) return <LoadingSpinner />;

  async function handleSave(e: FormEvent) {
    e.preventDefault();
    setError('');
    setSaving(true);
    try {
      await usersApi.update(user!.id, { email });
      await refresh();
      setEditing(false);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="profile-page">
      <div className="profile-card">
        <h1>个人资料</h1>

        <div className="profile-info">
          <div className="form-group">
            <label>用户名</label>
            <p>{user.username}</p>
          </div>

          {editing ? (
            <form onSubmit={handleSave}>
              {error && <div className="form-error">{error}</div>}
              <div className="form-group">
                <label htmlFor="email">邮箱</label>
                <input id="email" type="email" value={email} onChange={(e) => setEmail(e.target.value)} />
              </div>
              <div className="form-actions">
                <button type="submit" className="btn btn-primary" disabled={saving}>
                  {saving ? '保存中...' : '保存'}
                </button>
                <button type="button" className="btn" onClick={() => setEditing(false)}>取消</button>
              </div>
            </form>
          ) : (
            <>
              <div className="form-group">
                <label>邮箱</label>
                <p>{user.email || '未设置'}</p>
              </div>
              <div className="form-group">
                <label>角色</label>
                <p>{user.roles.join(', ') || '无'}</p>
              </div>
              <div className="form-actions">
                <button className="btn" onClick={() => setEditing(true)}>编辑</button>
                <button className="btn btn-danger" onClick={logout}>退出登录</button>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
