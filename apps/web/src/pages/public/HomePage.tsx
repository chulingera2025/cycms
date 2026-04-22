import { Card, Empty, Skeleton, Tag, Typography } from 'antd';
import { ArrowRight, BookOpenText, Sparkles } from 'lucide-react';
import { Link } from 'react-router-dom';
import type { BlogPostSummary } from '@/features/public/blog';
import {
  useBlogPosts,
  useBlogSiteSettings,
  useFeaturedBlogPosts,
} from '@/features/public/hooks';
import { useMedia } from '@/features/media/hooks';
import { PageSkeleton } from '@/components/shared/PageSkeleton';
import { formatDateTime, resolveMediaUrl } from '@/utils/format';

function PostCard({ post, compact = false }: { post: BlogPostSummary; compact?: boolean }) {
  const { data: cover } = useMedia(post.coverImageId);

  return (
    <Link
      to={`/blog/${post.slug}`}
      className="block h-full no-underline"
    >
      <Card
        hoverable
        className="h-full overflow-hidden"
        styles={{ body: { padding: compact ? 18 : 0 } }}
      >
        {!compact && (
          <div className="aspect-[16/9] overflow-hidden bg-surface-alt">
            {cover ? (
              <img
                src={resolveMediaUrl(cover.storage_path)}
                alt={post.title}
                className="h-full w-full object-cover"
              />
            ) : (
              <div className="flex h-full items-end bg-gradient-to-br from-brand/25 via-accent/15 to-surface px-5 py-4">
                <span className="max-w-xs text-xl font-semibold tracking-tight text-text">
                  {post.title}
                </span>
              </div>
            )}
          </div>
        )}

        <div className={compact ? '' : 'p-5'}>
          <div className="mb-3 flex items-center gap-2">
            {post.featured && <Tag color="gold">精选</Tag>}
            {post.publishedAt && (
              <span className="text-xs text-text-muted">{formatDateTime(post.publishedAt)}</span>
            )}
          </div>

          <Typography.Title level={compact ? 5 : 4} style={{ marginTop: 0, marginBottom: 12 }}>
            {post.title}
          </Typography.Title>

          {post.excerpt ? (
            <Typography.Paragraph
              type="secondary"
              style={{ marginBottom: 0 }}
              ellipsis={{ rows: compact ? 3 : 4 }}
            >
              {post.excerpt}
            </Typography.Paragraph>
          ) : (
            <Typography.Text type="secondary">继续阅读这篇文章</Typography.Text>
          )}
        </div>
      </Card>
    </Link>
  );
}

export default function HomePage() {
  const settingsQuery = useBlogSiteSettings();
  const featuredQuery = useFeaturedBlogPosts(3);
  const postsQuery = useBlogPosts(6);

  const settings = settingsQuery.data;
  const featuredPosts =
    settings?.featuredPosts.length
      ? settings.featuredPosts.slice(0, 3)
      : featuredQuery.data?.length
        ? featuredQuery.data
        : postsQuery.data?.slice(0, 3) ?? [];
  const recentPosts = postsQuery.data ?? [];
  const isLoading = settingsQuery.isLoading || featuredQuery.isLoading || postsQuery.isLoading;

  if (isLoading && recentPosts.length === 0) {
    return <PageSkeleton variant="list" />;
  }

  return (
    <div className="flex flex-col gap-12">
      <section className="relative overflow-hidden rounded-[28px] border border-border bg-[radial-gradient(circle_at_top_left,_rgba(59,130,246,0.22),_transparent_34%),linear-gradient(135deg,rgba(255,255,255,0.96),rgba(247,250,252,0.9))] px-6 py-12 sm:px-10 lg:px-12 lg:py-16 dark:bg-[radial-gradient(circle_at_top_left,_rgba(59,130,246,0.24),_transparent_32%),linear-gradient(135deg,rgba(15,23,42,0.98),rgba(15,23,42,0.9))]">
        <div className="absolute -right-16 top-0 h-40 w-40 rounded-full bg-accent/20 blur-3xl" />
        <div className="relative grid gap-8 lg:grid-cols-[minmax(0,1.3fr)_minmax(280px,0.7fr)] lg:items-end">
          <div>
            <div className="inline-flex items-center gap-2 rounded-full border border-brand/20 bg-brand/10 px-3 py-1 text-xs font-medium uppercase tracking-[0.18em] text-brand">
              <Sparkles size={14} />
              博客首页
            </div>
            <h1 className="mt-5 max-w-3xl text-4xl font-semibold tracking-tight text-text sm:text-5xl">
              {settings?.heroTitle ?? '把内容、页面和发布流程收束成一套可运营的系统'}
            </h1>
            <p className="mt-4 max-w-2xl text-base leading-7 text-text-secondary sm:text-lg">
              {settings?.heroSubtitle
                ?? settings?.tagline
                ?? '默认模型为博客预设：既有无头的能力，也有前台展示的能力，并支持插件拓展'}
            </p>
            <div className="mt-8 flex flex-wrap gap-3">
              <Link
                to="/blog"
                className="inline-flex items-center gap-2 rounded-full bg-brand px-5 py-2.5 text-sm font-medium text-white no-underline transition-colors hover:bg-brand-hover"
              >
                阅读文章
                <ArrowRight size={14} />
              </Link>
              <Link
                to="/admin/content?type=post"
                className="inline-flex items-center gap-2 rounded-full border border-border bg-surface px-5 py-2.5 text-sm font-medium text-text no-underline transition-colors hover:border-brand hover:text-brand"
              >
                管理内容
                <BookOpenText size={14} />
              </Link>
            </div>
          </div>

          <Card styles={{ body: { padding: 20 } }}>
            <div className="text-xs uppercase tracking-[0.18em] text-text-muted">站点概览</div>
            <div className="mt-4 space-y-4">
              <div>
                <div className="text-2xl font-semibold text-text">{settings?.siteName ?? 'CyCMS Blog'}</div>
                <div className="mt-1 text-sm text-text-secondary">
                  {settings?.tagline ?? '结构化内容、页面和发布节奏统一管理。'}
                </div>
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div className="rounded-2xl bg-surface-alt p-4">
                  <div className="text-xs text-text-muted">精选文章</div>
                  <div className="mt-2 text-2xl font-semibold text-text">{featuredPosts.length}</div>
                </div>
                <div className="rounded-2xl bg-surface-alt p-4">
                  <div className="text-xs text-text-muted">最新文章</div>
                  <div className="mt-2 text-2xl font-semibold text-text">{recentPosts.length}</div>
                </div>
              </div>

              <div className="rounded-2xl border border-dashed border-border px-4 py-3 text-sm text-text-secondary">
                {settings?.footerText ?? '前台现在优先服务阅读流，通用内容可访问次级目录或api接口'}
              </div>
            </div>
          </Card>
        </div>
      </section>

      <section>
        <div className="mb-5 flex items-end justify-between gap-4">
          <div>
            <Typography.Title level={3} style={{ marginTop: 0, marginBottom: 4 }}>
              精选文章
            </Typography.Title>
            <Typography.Text type="secondary">
              首页优先展示推荐内容
            </Typography.Text>
          </div>
          <Link
            to="/blog"
            className="inline-flex items-center gap-1 text-sm font-medium text-brand no-underline"
          >
            查看全部文章
            <ArrowRight size={14} />
          </Link>
        </div>

        {featuredQuery.isLoading && featuredPosts.length === 0 ? (
          <Skeleton active paragraph={{ rows: 6 }} />
        ) : featuredPosts.length === 0 ? (
          <Empty description="还没有可展示的精选文章" />
        ) : (
          <div className="grid grid-cols-1 gap-5 lg:grid-cols-3">
            {featuredPosts.map((post) => (
              <PostCard key={post.id} post={post} />
            ))}
          </div>
        )}
      </section>

      <section>
        <div className="mb-5 flex items-end justify-between gap-4">
          <div>
            <Typography.Title level={3} style={{ marginTop: 0, marginBottom: 4 }}>
              最新发布
            </Typography.Title>
            <Typography.Text type="secondary">
              新文章直接作为首页主内容流
            </Typography.Text>
          </div>
        </div>

        {postsQuery.isLoading && recentPosts.length === 0 ? (
          <PageSkeleton variant="list" />
        ) : recentPosts.length === 0 ? (
          <Empty description="还没有已发布文章，先去后台创建一篇 post 内容吧。" />
        ) : (
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
            {recentPosts.map((post) => (
              <PostCard key={post.id} post={post} compact />
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
