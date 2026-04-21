import { useState, type FormEvent } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { publicApi } from '@/lib/api';
import { useAuth } from '@/stores/auth';
import { ApiError } from '@/lib/api/client';

export default function MemberLoginPage() {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const { refresh } = useAuth();

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setError('');
    setSubmitting(true);
    try {
      await publicApi.login({ username, password });
      await refresh();
      navigate(searchParams.get('redirect') ?? '/');
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '登录失败');
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="auth-page">
      <div className="auth-card">
        <h1>会员登录</h1>
        <form onSubmit={handleSubmit}>
          {error && <div className="form-error">{error}</div>}
          <div className="form-group">
            <label htmlFor="username">用户名</label>
            <input id="username" value={username} onChange={(e) => setUsername(e.target.value)} required autoComplete="username" />
          </div>
          <div className="form-group">
            <label htmlFor="password">密码</label>
            <input id="password" type="password" value={password} onChange={(e) => setPassword(e.target.value)} required autoComplete="current-password" />
          </div>
          <button type="submit" disabled={submitting} className="btn btn-primary">
            {submitting ? '登录中...' : '登录'}
          </button>
        </form>
        <p className="auth-link">
          没有账号？<a href="/register">注册</a>
        </p>
      </div>
    </div>
  );
}
