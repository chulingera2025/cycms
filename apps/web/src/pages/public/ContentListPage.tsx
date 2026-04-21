import { useState, useMemo } from 'react';
import { useParams, Link } from 'react-router-dom';
import { useAsync } from '@/hooks/useAsync';
import { publicApi } from '@/lib/api';
import { LoadingSpinner } from '@/components/shared/LoadingSpinner';

export default function ContentListPage() {
  const { typeApiId } = useParams<{ typeApiId: string }>();
  const [page, setPage] = useState(1);
  const params = useMemo(
    () => ({ page: String(page), pageSize: '12' }),
    [page],
  );

  const { data, loading, error } = useAsync(
    () => (typeApiId ? publicApi.listContent(typeApiId, params) : Promise.resolve(null)),
    [typeApiId, params],
  );

  if (!typeApiId) return <div>请选择内容类型</div>;
  if (loading) return <LoadingSpinner />;
  if (error) return <div className="page-error">加载失败: {error.message}</div>;

  return (
    <div className="content-list-page">
      <h1 style={{ textTransform: 'capitalize' }}>{typeApiId.replace(/_/g, ' ')}</h1>

      <div className="content-grid">
        {data?.data.map((entry) => (
          <Link
            key={entry.id}
            to={`/content/${typeApiId}/${entry.slug ?? entry.id}`}
            className="content-card"
          >
            <h3>{entry.slug ?? entry.id.slice(0, 8)}</h3>
            <time>{new Date(entry.published_at ?? entry.created_at).toLocaleDateString()}</time>
          </Link>
        ))}
        {data?.data.length === 0 && <p>暂无已发布的内容</p>}
      </div>

      {data && data.meta.page_count > 1 && (
        <div className="pagination">
          <button disabled={page <= 1} onClick={() => setPage(page - 1)}>上一页</button>
          <span>{data.meta.page} / {data.meta.page_count}</span>
          <button disabled={page >= data.meta.page_count} onClick={() => setPage(page + 1)}>下一页</button>
        </div>
      )}
    </div>
  );
}
