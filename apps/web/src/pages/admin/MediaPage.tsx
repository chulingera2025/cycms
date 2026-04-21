import { useState, useRef, useMemo } from 'react';
import { useAsync } from '@/hooks/useAsync';
import { mediaApi } from '@/lib/api';
import { LoadingSpinner } from '@/components/shared/LoadingSpinner';

export default function MediaPage() {
  const [page, setPage] = useState(1);
  const [mimeFilter, setMimeFilter] = useState('');
  const fileRef = useRef<HTMLInputElement>(null);
  const [uploading, setUploading] = useState(false);

  const params = useMemo(() => {
    const p: Record<string, string> = { page: String(page), pageSize: '20' };
    if (mimeFilter) p['mime_type'] = mimeFilter;
    return p;
  }, [page, mimeFilter]);

  const { data, loading, refetch } = useAsync(
    () => mediaApi.list(params),
    [params],
  );

  async function handleUpload() {
    const file = fileRef.current?.files?.[0];
    if (!file) return;
    setUploading(true);
    try {
      await mediaApi.upload(file);
      if (fileRef.current) fileRef.current.value = '';
      refetch();
    } finally {
      setUploading(false);
    }
  }

  return (
    <div className="page">
      <div className="page-header">
        <h1>媒体管理</h1>
        <div className="header-actions">
          <select value={mimeFilter} onChange={(e) => { setMimeFilter(e.target.value); setPage(1); }}>
            <option value="">全部类型</option>
            <option value="image/jpeg">JPEG</option>
            <option value="image/png">PNG</option>
            <option value="image/webp">WebP</option>
            <option value="application/pdf">PDF</option>
          </select>
          <input ref={fileRef} type="file" />
          <button className="btn btn-primary" onClick={handleUpload} disabled={uploading}>
            {uploading ? '上传中...' : '上传'}
          </button>
        </div>
      </div>

      {loading ? (
        <LoadingSpinner />
      ) : (
        <>
          <div className="media-grid">
            {data?.data.map((asset) => (
              <div key={asset.id} className="media-card">
                {asset.mime_type.startsWith('image/') ? (
                  <img
                    src={asset.storage_path.startsWith('/') ? asset.storage_path : `/uploads/${asset.storage_path}`}
                    alt={asset.original_filename}
                    className="media-preview"
                  />
                ) : (
                  <div className="media-file-icon">{asset.mime_type.split('/')[1]?.toUpperCase()}</div>
                )}
                <div className="media-info">
                  <span className="media-filename" title={asset.original_filename}>
                    {asset.original_filename}
                  </span>
                  <span className="media-size">{formatSize(asset.size)}</span>
                </div>
                <div className="media-actions">
                  <button
                    className="btn btn-sm"
                    onClick={() => {
                      const url = asset.storage_path.startsWith('/') ? asset.storage_path : `/uploads/${asset.storage_path}`;
                      navigator.clipboard.writeText(window.location.origin + url);
                    }}
                  >
                    复制链接
                  </button>
                  <button
                    className="btn btn-sm btn-danger"
                    onClick={async () => {
                      if (confirm(`确定删除 ${asset.original_filename}？`)) {
                        await mediaApi.delete(asset.id);
                        refetch();
                      }
                    }}
                  >
                    删除
                  </button>
                </div>
              </div>
            ))}
          </div>

          {data && data.page_count > 1 && (
            <div className="pagination">
              <button disabled={page <= 1} onClick={() => setPage(page - 1)}>上一页</button>
              <span>{data.page} / {data.page_count}（共 {data.total} 条）</span>
              <button disabled={page >= data.page_count} onClick={() => setPage(page + 1)}>下一页</button>
            </div>
          )}
        </>
      )}
    </div>
  );
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
