import { useMemo, useState } from 'react';
import { Card, Empty, Pagination, Tag, Typography } from 'antd';
import { Link, useLocation, useParams } from 'react-router-dom';
import type { BlogPostSummary } from '@/features/public/blog';
import { useMedia } from '@/features/media/hooks';
import { useBlogPostIndex, usePublicContentList } from '@/features/public/hooks';
import { PageSkeleton } from '@/components/shared/PageSkeleton';
import { formatDateTime, resolveMediaUrl } from '@/utils/format';

function BlogListCard({ post }: { post: BlogPostSummary }) {
  const { data: cover } = useMedia(post.coverImageId);

  return (
    <Link to={`/blog/${post.slug}`} className="block h-full no-underline">
      <Card hoverable className="h-full overflow-hidden" styles={{ body: { padding: 0 } }}>
        <div className="aspect-[16/10] overflow-hidden bg-surface-alt">
          {cover ? (
            <img
              src={resolveMediaUrl(cover.storage_path)}
              alt={post.title}
              className="h-full w-full object-cover"
            />
          ) : (
            <div className="flex h-full items-end bg-gradient-to-br from-brand/20 via-accent/10 to-surface px-5 py-4">
              <span className="max-w-xs text-xl font-semibold tracking-tight text-text">
                {post.title}
              </span>
            </div>
          )}
        </div>

        <div className="p-5">
          <div className="mb-3 flex items-center gap-2">
            {post.featured && <Tag color="gold">精选</Tag>}
            {post.publishedAt && (
              <span className="text-xs text-text-muted">{formatDateTime(post.publishedAt)}</span>
            )}
          </div>
          <Typography.Title level={4} style={{ marginTop: 0, marginBottom: 12 }}>
            {post.title}
          </Typography.Title>
          <Typography.Paragraph type="secondary" style={{ marginBottom: 0 }} ellipsis={{ rows: 3 }}>
            {post.excerpt ?? '继续阅读这篇文章。'}
          </Typography.Paragraph>
        </div>
      </Card>
    </Link>
  );
}

export default function ContentListPage() {
  const { typeApiId } = useParams<{ typeApiId: string }>();
  const location = useLocation();
  const [page, setPage] = useState(1);
  const pageSize = 12;
  const effectiveTypeApiId = typeApiId ?? (location.pathname.startsWith('/blog') ? 'post' : undefined);
  const isBlogMode = effectiveTypeApiId === 'post';

  const params = useMemo(
    () => ({ page: String(page), pageSize: String(pageSize) }),
    [page],
  );

  const { data, isLoading } = usePublicContentList(isBlogMode ? undefined : effectiveTypeApiId, params);
  const blogQuery = useBlogPostIndex(page, pageSize, isBlogMode);

  if (!effectiveTypeApiId) return <Empty description="请选择内容类型" />;
  if ((isBlogMode && blogQuery.isLoading) || (!isBlogMode && isLoading)) {
    return <PageSkeleton variant="list" />;
  }

  if (isBlogMode) {
    const entries = blogQuery.data?.data ?? [];
    const meta = blogQuery.data?.meta;

    return (
      <div className="flex flex-col gap-6">
        <section className="rounded-[24px] border border-border bg-gradient-to-br from-brand/10 via-surface to-accent/10 px-6 py-10">
          <Typography.Title level={2} style={{ marginTop: 0, marginBottom: 8 }}>
            博客文章
          </Typography.Title>
          <Typography.Paragraph type="secondary" style={{ marginBottom: 0 }}>
            这里展示博客下已发布的文章内容，按发布时间倒序排列。
          </Typography.Paragraph>
        </section>

        {entries.length === 0 ? (
          <Empty description="暂无已发布文章" />
        ) : (
          <div className="grid grid-cols-1 gap-5 md:grid-cols-2 xl:grid-cols-3">
            {entries.map((post) => (
              <BlogListCard key={post.id} post={post} />
            ))}
          </div>
        )}

        {meta && meta.page_count > 1 && (
          <div className="mt-2 flex justify-center">
            <Pagination
              current={page}
              pageSize={pageSize}
              total={meta.total}
              onChange={setPage}
              showSizeChanger={false}
            />
          </div>
        )}
      </div>
    );
  }

  return (
    <div>
      <Typography.Title level={2} style={{ marginTop: 0, textTransform: 'capitalize' }}>
        {effectiveTypeApiId.replace(/_/g, ' ')}
      </Typography.Title>

      {data && data.data.length === 0 ? (
        <Empty description="暂无已发布的内容" />
      ) : (
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {data?.data.map((entry) => (
            <Link
              key={entry.id}
              to={`/content/${effectiveTypeApiId}/${entry.slug ?? entry.id}`}
              className="block no-underline"
            >
              <Card hoverable>
                <Typography.Title level={5} style={{ marginTop: 0 }}>
                  {entry.slug ?? entry.id.slice(0, 8)}
                </Typography.Title>
                <Typography.Text type="secondary" className="text-xs">
                  {formatDateTime(entry.published_at ?? entry.created_at)}
                </Typography.Text>
              </Card>
            </Link>
          ))}
        </div>
      )}

      {data && data.meta.page_count > 1 && (
        <div className="mt-6 flex justify-center">
          <Pagination
            current={page}
            pageSize={pageSize}
            total={data.meta.total}
            onChange={setPage}
            showSizeChanger={false}
          />
        </div>
      )}
    </div>
  );
}
