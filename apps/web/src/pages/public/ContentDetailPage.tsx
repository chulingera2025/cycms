import { Breadcrumb, Collapse, Empty, Tag, Typography } from 'antd';
import { Link, useParams } from 'react-router-dom';
import { usePublicContentDetail } from '@/features/public/hooks';
import { PageSkeleton } from '@/components/shared/PageSkeleton';
import { formatDateTime } from '@/utils/format';

// TODO!!! richtext 字段当前作为纯文本展示；后续引入 markdown 渲染（react-markdown 或内联 MDEditor.Markdown）
function FieldValue({ value }: { value: unknown }) {
  if (value == null || value === '') {
    return <Typography.Text type="secondary">—</Typography.Text>;
  }
  if (typeof value === 'string') {
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
  const { data: entry, isLoading, error } = usePublicContentDetail(typeApiId, idOrSlug);

  if (isLoading) return <PageSkeleton variant="detail" />;
  if (error || !entry) return <Empty description="内容未找到" />;

  return (
    <article>
      <Breadcrumb
        items={[
          { title: <Link to="/">首页</Link> },
          { title: <Link to={`/content/${typeApiId}`}>{typeApiId}</Link> },
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
