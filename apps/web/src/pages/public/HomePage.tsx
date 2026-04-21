import { Card, Empty, Typography } from 'antd';
import { ArrowRight } from 'lucide-react';
import { Link } from 'react-router-dom';
import { usePublicContentTypes } from '@/features/public/hooks';
import { PageSkeleton } from '@/components/shared/PageSkeleton';

export default function HomePage() {
  const { data: types = [], isLoading } = usePublicContentTypes();

  return (
    <div className="flex flex-col gap-10">
      <section className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-brand/10 via-surface to-accent/10 px-6 py-14 sm:px-10">
        <h1 className="m-0 text-3xl font-bold tracking-tight text-text sm:text-4xl">
          欢迎来到 CyCMS
        </h1>
        <p className="mt-3 max-w-2xl text-base text-text-secondary">
          一个灵活、可扩展的 Headless CMS —— 自定义内容类型、丰富字段、版本管理、权限与插件。
        </p>
        <div className="mt-6 flex flex-wrap gap-3">
          <Link
            to="/content"
            className="inline-flex items-center gap-1 rounded bg-brand px-5 py-2 text-sm font-medium text-white no-underline transition-colors hover:bg-brand-hover"
          >
            浏览内容
            <ArrowRight size={14} />
          </Link>
          <Link
            to="/register"
            className="inline-flex items-center rounded border border-border bg-surface px-5 py-2 text-sm font-medium text-text no-underline transition-colors hover:border-brand hover:text-brand"
          >
            注册会员
          </Link>
        </div>
      </section>

      <section>
        <Typography.Title level={3} style={{ marginTop: 0 }}>
          浏览内容
        </Typography.Title>
        {isLoading ? (
          <PageSkeleton variant="list" />
        ) : types.length === 0 ? (
          <Empty description="暂无可浏览内容" />
        ) : (
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {types.map((ct) => (
              <Link
                key={ct.id}
                to={`/content/${ct.api_id}`}
                className="block no-underline"
              >
                <Card hoverable styles={{ body: { padding: 20 } }}>
                  <Typography.Title level={4} style={{ marginTop: 0 }}>
                    {ct.name}
                  </Typography.Title>
                  {ct.description ? (
                    <Typography.Paragraph
                      type="secondary"
                      style={{ marginBottom: 0 }}
                      ellipsis={{ rows: 2 }}
                    >
                      {ct.description}
                    </Typography.Paragraph>
                  ) : (
                    <Typography.Text type="secondary">
                      <code className="font-mono text-xs">/{ct.api_id}</code>
                    </Typography.Text>
                  )}
                </Card>
              </Link>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
