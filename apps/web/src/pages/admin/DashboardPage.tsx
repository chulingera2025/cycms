import { useAuth } from '@/stores/auth';
import { useAsync } from '@/hooks/useAsync';
import { contentTypesApi, usersApi } from '@/lib/api';
import { LoadingSpinner } from '@/components/shared/LoadingSpinner';

export default function DashboardPage() {
  const { user } = useAuth();
  const types = useAsync(() => contentTypesApi.list(), []);
  const users = useAsync(() => usersApi.list(), []);

  return (
    <div className="page">
      <h1>仪表盘</h1>
      <p>欢迎回来，{user?.username}！</p>
      <div className="dashboard-grid">
        <DashboardCard
          title="内容类型"
          loading={types.loading}
          value={types.data?.length}
          link="/admin/content-types"
        />
        <DashboardCard
          title="用户数"
          loading={users.loading}
          value={users.data?.length}
          link="/admin/users"
        />
      </div>
    </div>
  );
}

function DashboardCard({
  title,
  loading,
  value,
  link,
}: {
  title: string;
  loading: boolean;
  value?: number;
  link: string;
}) {
  return (
    <a href={link} className="dashboard-card">
      <h3>{title}</h3>
      {loading ? <LoadingSpinner /> : <span className="card-value">{value ?? 0}</span>}
    </a>
  );
}
