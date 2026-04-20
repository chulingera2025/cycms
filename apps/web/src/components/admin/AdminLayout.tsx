import { NavLink, Outlet, useNavigate } from 'react-router-dom';
import { useAuth } from '@/stores/auth';
import { authApi } from '@/api';

const NAV_ITEMS = [
  { to: '/admin/dashboard', label: '仪表盘', icon: '📊' },
  { to: '/admin/content-types', label: '内容类型', icon: '📋' },
  { to: '/admin/content', label: '内容管理', icon: '📝' },
  { to: '/admin/media', label: '媒体管理', icon: '🖼️' },
  { to: '/admin/plugins', label: '插件管理', icon: '🧩' },
  { to: '/admin/users', label: '用户管理', icon: '👤' },
  { to: '/admin/roles', label: '角色权限', icon: '🔐' },
  { to: '/admin/settings', label: '系统设置', icon: '⚙️' },
];

export default function AdminLayout() {
  const { user } = useAuth();
  const navigate = useNavigate();

  function handleLogout() {
    authApi.logout();
    navigate('/admin/login');
  }

  return (
    <div className="admin-layout">
      <aside className="admin-sidebar">
        <div className="sidebar-header">
          <h2>CyCMS</h2>
        </div>
        <nav className="sidebar-nav">
          {NAV_ITEMS.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              end={false}
              className={({ isActive }) =>
                `nav-item${isActive ? ' active' : ''}`
              }
            >
              <span className="nav-icon">{item.icon}</span>
              <span className="nav-label">{item.label}</span>
            </NavLink>
          ))}
        </nav>
        <div className="sidebar-footer">
          <span className="user-label">{user?.username}</span>
          <button onClick={handleLogout} className="btn btn-text">
            退出
          </button>
        </div>
      </aside>
      <main className="admin-main">
        <Outlet />
      </main>
    </div>
  );
}
