import { useState, useMemo } from 'react';
import { Link } from 'react-router-dom';
import { useAsync } from '@/hooks/useAsync';
import { publicApi } from '@/api';
import { LoadingSpinner } from '@/components/shared/LoadingSpinner';
import type { PublicContentType } from '@/types';

export default function SearchPage() {
  const [query, setQuery] = useState('');
  const [submitted, setSubmitted] = useState('');
  const { data: types } = useAsync(() => publicApi.listContentTypes(), []);
  const [selectedType, setSelectedType] = useState<string>('');

  const typeApiId = selectedType || types?.[0]?.api_id || '';

  const params = useMemo(() => {
    if (!submitted) return null;
    return { 'filter[slug][contains]': submitted, pageSize: '20' };
  }, [submitted]);

  const { data, loading } = useAsync(
    () =>
      params && typeApiId
        ? publicApi.listContent(typeApiId, params)
        : Promise.resolve(null),
    [typeApiId, params],
  );

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setSubmitted(query);
  }

  return (
    <div className="search-page">
      <h1>搜索</h1>
      <form onSubmit={handleSubmit} className="search-form">
        <select value={selectedType} onChange={(e) => setSelectedType(e.target.value)}>
          {types?.map((t: PublicContentType) => (
            <option key={t.api_id} value={t.api_id}>{t.name}</option>
          ))}
        </select>
        <input
          type="search"
          placeholder="搜索..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <button type="submit" className="btn btn-primary">搜索</button>
      </form>

      {loading && <LoadingSpinner />}

      {data && (
        <div className="search-results">
          <p>共 {data.meta.total} 条结果</p>
          <div className="content-grid">
            {data.data.map((entry) => (
              <Link
                key={entry.id}
                to={`/content/${typeApiId}/${entry.slug ?? entry.id}`}
                className="content-card"
              >
                <h3>{entry.slug ?? entry.id.slice(0, 8)}</h3>
                <time>{new Date(entry.published_at ?? entry.created_at).toLocaleDateString()}</time>
              </Link>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
