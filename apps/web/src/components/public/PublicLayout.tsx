import { NavLink, Outlet, Link, useLocation } from 'react-router-dom';
import { useAuth } from '@/stores/auth';

export default function PublicLayout() {
  const { user } = useAuth();
  const location = useLocation();

  return (
    <div className="public-layout">
      <header className="public-header">
        <div className="header-inner">
          <Link to="/" className="site-logo">CyCMS</Link>
          <nav className="public-nav">
            <NavLink to="/" end>首页</NavLink>
            <NavLink to="/content">内容</NavLink>
            <NavLink to="/search">搜索</NavLink>
          </nav>
          <div className="header-auth">
            {user ? (
              <>
                <Link to="/profile">{user.username}</Link>
                <Link to="/admin" className="btn btn-sm" style={{ marginLeft: 8 }}>
                  管理后台
                </Link>
              </>
            ) : (
              <>
                <Link to={`/login?redirect=${encodeURIComponent(location.pathname)}`}>登录</Link>
                <Link to="/register">注册</Link>
              </>
            )}
          </div>
        </div>
      </header>

      <main className="public-main">
        <Outlet />
      </main>

      <footer className="public-footer">
        <div className="footer-inner">
          <p>&copy; {new Date().getFullYear()} CyCMS. All rights reserved.</p>
        </div>
      </footer>
    </div>
  );
}
