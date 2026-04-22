import { Suspense, lazy } from 'react';
import { Breadcrumb, Collapse, Empty, Skeleton, Tag, Typography } from 'antd';
import { Link, useLocation, useParams } from 'react-router-dom';
import { useMedia } from '@/features/media/hooks';
import { useBlogPostDetail, usePublicContentDetail } from '@/features/public/hooks';
import { PageSkeleton } from '@/components/shared/PageSkeleton';
import { formatDateTime, resolveMediaUrl } from '@/utils/format';

const MarkdownPreview = lazy(async () => {
  const mod = await import('@uiw/react-md-editor');
  return { default: mod.default.Markdown };
});

const MARKDOWN_HINT = /(^|\n)\s{0,3}(#{1,6}\s|>\s|[-*+]\s|\d+\.\s|```)|\*\*|__|~~|\[[^\]]+\]\([^)]+\)|`[^`]+`/;

function looksLikeMarkdown(source: string): boolean {
  if (source.includes('\n\n')) return true;
  return MARKDOWN_HINT.test(source);
}

function FieldValue({ value }: { value: unknown }) {
  if (value == null || value === '') {
    return <Typography.Text type="secondary">—</Typography.Text>;
  }
  if (typeof value === 'string') {
    if (looksLikeMarkdown(value)) {
      return (
        <div data-color-mode="inherit" className="prose prose-slate max-w-none dark:prose-invert">
          <Suspense fallback={<Skeleton active paragraph={{ rows: 3 }} />}>
            <MarkdownPreview
              source={value}
              style={{ background: 'transparent', color: 'inherit' }}
            />
          </Suspense>
        </div>
      );
    }
    return (
      <Typography.Paragraph style={{ whiteSpace: 'pre-wrap', marginBottom: 0 }}>
        {value}
      </Typography.Paragraph>
    );
  }
  if (typeof value === 'boolean') return <Tag>{value ? 'true' : 'false'}</Tag>;
  if (typeof value === 'number') return <span>{value}</span>;
  return (
    <pre className="m-0 overflow-auto rounded bg-surface-alt p-3 font-mono text-xs text-text">
      {JSON.stringify(value, null, 2)}
    </pre>
  );
}

export default function ContentDetailPage() {
  const { typeApiId, idOrSlug } = useParams<{ typeApiId: string; idOrSlug: string }>();
  const location = useLocation();
  const effectiveTypeApiId = typeApiId ?? (location.pathname.startsWith('/blog') ? 'post' : undefined);
  const isBlogMode = effectiveTypeApiId === 'post';
  const { data: entry, isLoading, error } = usePublicContentDetail(
    isBlogMode ? undefined : effectiveTypeApiId,
    idOrSlug,
  );
  const blogQuery = useBlogPostDetail(idOrSlug, isBlogMode);
  const { data: cover } = useMedia(blogQuery.data?.coverImageId);

  if ((isBlogMode && blogQuery.isLoading) || (!isBlogMode && isLoading)) {
    return <PageSkeleton variant="detail" />;
  }

  if (isBlogMode) {
    const post = blogQuery.data;

    if (blogQuery.error || !post) {
      return <Empty description="文章未找到" />;
    }

    return (
      <article className="mx-auto max-w-4xl">
        <Breadcrumb
          items={[
            { title: <Link to="/">首页</Link> },
            { title: <Link to="/blog">博客</Link> },
            { title: post.title },
          ]}
        />

        <header className="mt-5">
          <div className="mb-4 flex flex-wrap items-center gap-2">
            {post.featured && <Tag color="gold">精选</Tag>}
            {post.categories.map((category) => (
              <Tag key={category.id} color="blue">{category.name}</Tag>
            ))}
            {post.tags.map((tag) => (
              <Tag key={tag.id}>{tag.name}</Tag>
            ))}
          </div>

          <Typography.Title level={1} style={{ marginTop: 0, marginBottom: 12 }}>
            {post.title}
          </Typography.Title>
          {post.excerpt && (
            <Typography.Paragraph type="secondary" style={{ fontSize: 16, marginBottom: 12 }}>
              {post.excerpt}
            </Typography.Paragraph>
          )}
          {post.publishedAt && (
            <Typography.Text type="secondary">
              发布于 {formatDateTime(post.publishedAt)}
            </Typography.Text>
          )}
        </header>

        {cover && (
          <div className="mt-6 overflow-hidden rounded-[24px] border border-border bg-surface-alt">
            <img
              src={resolveMediaUrl(cover.storage_path)}
              alt={post.title}
              className="h-auto w-full object-cover"
            />
          </div>
        )}

        <section className="mt-8 rounded-[24px] border border-border bg-surface px-6 py-7 shadow-sm">
          <div data-color-mode="inherit" className="prose prose-slate max-w-none dark:prose-invert">
            {looksLikeMarkdown(post.body) ? (
              <Suspense fallback={<Skeleton active paragraph={{ rows: 8 }} />}>
                <MarkdownPreview
                  source={post.body}
                  style={{ background: 'transparent', color: 'inherit' }}
                />
              </Suspense>
            ) : (
              <Typography.Paragraph style={{ whiteSpace: 'pre-wrap', marginBottom: 0 }}>
                {post.body}
              </Typography.Paragraph>
            )}
          </div>
        </section>
      </article>
    );
  }

  if (error || !entry || !effectiveTypeApiId) return <Empty description="内容未找到" />;

  return (
    <article>
      <Breadcrumb
        items={[
          { title: <Link to="/">首页</Link> },
          { title: <Link to={`/content/${effectiveTypeApiId}`}>{effectiveTypeApiId}</Link> },
          { title: entry.slug ?? entry.id.slice(0, 8) },
        ]}
      />
      <Typography.Title level={1} style={{ marginTop: 16 }}>
        {entry.slug ?? entry.id.slice(0, 8)}
      </Typography.Title>
      <Typography.Text type="secondary">
        发布于 {formatDateTime(entry.published_at ?? entry.created_at)}
      </Typography.Text>

      <div className="mt-8 flex flex-col gap-6">
        {Object.entries(entry.fields as Record<string, unknown>).map(([key, value]) => (
          <section
            key={key}
            className="rounded-lg border border-border bg-surface p-5"
          >
            <Typography.Title level={4} style={{ marginTop: 0 }}>
              {key}
            </Typography.Title>
            <FieldValue value={value} />
          </section>
        ))}
      </div>

      {entry.populated && Object.keys(entry.populated).length > 0 && (
        <Collapse
          className="mt-6"
          items={Object.entries(entry.populated).map(([key, arr]) => ({
            key,
            label: `${key} (${arr.length})`,
            children: (
              <ul className="m-0 pl-4">
                {arr.map((e) => (
                  <li key={e.id}>
                    <Link to={`/content/${e.content_type_api_id}/${e.slug ?? e.id}`}>
                      {e.slug ?? e.id}
                    </Link>
                  </li>
                ))}
              </ul>
            ),
          }))}
        />
      )}
    </article>
  );
}
