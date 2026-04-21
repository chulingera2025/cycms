import { useState, type FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { publicApi } from '@/lib/api';
import { ApiError } from '@/lib/api/client';

export default function MemberRegisterPage() {
  const [username, setUsername] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [error, setError] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const navigate = useNavigate();

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setError('');
    if (password !== confirm) {
      setError('两次输入的密码不一致');
      return;
    }
    setSubmitting(true);
    try {
      await publicApi.register({ username, email, password });
      navigate('/login');
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '注册失败');
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="auth-page">
      <div className="auth-card">
        <h1>会员注册</h1>
        <form onSubmit={handleSubmit}>
          {error && <div className="form-error">{error}</div>}
          <div className="form-group">
            <label htmlFor="username">用户名</label>
            <input id="username" value={username} onChange={(e) => setUsername(e.target.value)} required autoComplete="username" />
          </div>
          <div className="form-group">
            <label htmlFor="email">邮箱</label>
            <input id="email" type="email" value={email} onChange={(e) => setEmail(e.target.value)} required autoComplete="email" />
          </div>
          <div className="form-group">
            <label htmlFor="password">密码</label>
            <input id="password" type="password" value={password} onChange={(e) => setPassword(e.target.value)} required autoComplete="new-password" />
          </div>
          <div className="form-group">
            <label htmlFor="confirm">确认密码</label>
            <input id="confirm" type="password" value={confirm} onChange={(e) => setConfirm(e.target.value)} required autoComplete="new-password" />
          </div>
          <button type="submit" disabled={submitting} className="btn btn-primary">
            {submitting ? '注册中...' : '注册'}
          </button>
        </form>
        <p className="auth-link">
          已有账号？<a href="/login">登录</a>
        </p>
      </div>
    </div>
  );
}
