import { useParams, Link } from 'react-router-dom';
import { useAsync } from '@/hooks/useAsync';
import { publicApi } from '@/lib/api';
import { LoadingSpinner } from '@/components/shared/LoadingSpinner';

export default function ContentDetailPage() {
  const { typeApiId, idOrSlug } = useParams<{ typeApiId: string; idOrSlug: string }>();

  const { data: entry, loading, error } = useAsync(
    () =>
      typeApiId && idOrSlug
        ? publicApi.getContent(typeApiId, idOrSlug)
        : Promise.resolve(null),
    [typeApiId, idOrSlug],
  );

  if (loading) return <LoadingSpinner />;
  if (error) return <div className="page-error">内容未找到</div>;
  if (!entry) return null;

  return (
    <article className="content-detail-page">
      <nav className="breadcrumb">
        <Link to="/">首页</Link> /{' '}
        <Link to={`/content/${typeApiId}`}>{typeApiId}</Link> /{' '}
        <span>{entry.slug ?? entry.id.slice(0, 8)}</span>
      </nav>

      <header>
        <h1>{entry.slug ?? entry.id}</h1>
        <time>{new Date(entry.published_at ?? entry.created_at).toLocaleDateString()}</time>
      </header>

      <div className="entry-fields">
        {Object.entries(entry.fields as Record<string, unknown>).map(([key, value]) => (
          <div key={key} className="field-block">
            <h3>{key}</h3>
            <div className="field-value">
              {typeof value === 'string' ? (
                <div dangerouslySetInnerHTML={{ __html: value }} />
              ) : (
                <pre>{JSON.stringify(value, null, 2)}</pre>
              )}
            </div>
          </div>
        ))}
      </div>
    </article>
  );
}
