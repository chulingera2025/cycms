import { Link } from 'react-router-dom';
import { useAsync } from '@/hooks/useAsync';
import { publicApi } from '@/api';
import { LoadingSpinner } from '@/components/shared/LoadingSpinner';

export default function ContentIndexPage() {
  const { data: types, loading } = useAsync(() => publicApi.listContentTypes(), []);

  if (loading) return <LoadingSpinner />;

  return (
    <div className="content-index-page">
      <h1>全部内容</h1>
      <div className="content-type-grid">
        {types?.map((ct) => (
          <Link key={ct.id} to={`/content/${ct.api_id}`} className="content-type-card">
            <h3>{ct.name}</h3>
            {ct.description && <p>{ct.description}</p>}
          </Link>
        ))}
      </div>
    </div>
  );
}
