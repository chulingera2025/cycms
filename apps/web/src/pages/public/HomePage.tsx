import { Link } from 'react-router-dom';
import { useAsync } from '@/hooks/useAsync';
import { publicApi } from '@/api';
import { LoadingSpinner } from '@/components/shared/LoadingSpinner';

export default function HomePage() {
  const { data: types, loading } = useAsync(() => publicApi.listContentTypes(), []);

  return (
    <div className="home-page">
      <section className="hero">
        <h1>欢迎来到 CyCMS</h1>
        <p>一个灵活的内容管理系统</p>
      </section>

      <section className="content-sections">
        <h2>浏览内容</h2>
        {loading ? (
          <LoadingSpinner />
        ) : (
          <div className="content-type-grid">
            {types?.map((ct) => (
              <Link key={ct.id} to={`/content/${ct.api_id}`} className="content-type-card">
                <h3>{ct.name}</h3>
                {ct.description && <p>{ct.description}</p>}
              </Link>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
